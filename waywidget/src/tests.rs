#[cfg(test)]
mod tests {
    use crate::{apply_ops_to_svg, find_element_by_id, SvgOp, WidgetAPI, WidgetState, ElementHandle, JsContext, get_proto};
    use xmltree::Element;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::path::PathBuf;
    use boa_engine::{Source, JsString, JsValue, JsObject};

    #[test]
    fn test_class_management() {
        let svg = r#"<svg><rect id="test" class="foo bar" /></svg>"#;
        let mut root = Element::parse(svg.as_bytes()).unwrap();
        let mut ops = HashMap::new();
        ops.insert("test".to_string(), vec![SvgOp::AddClass("baz".to_string()), SvgOp::RemoveClass("foo".to_string())]);
        apply_ops_to_svg(&mut root, ops);
        let el = find_element_by_id(&mut root, "test").unwrap();
        let classes = el.attributes.get("class").unwrap();
        assert!(classes.contains("bar"));
        assert!(classes.contains("baz"));
        assert!(!classes.contains("foo"));
    }

    #[test]
    fn test_visibility_and_opacity() {
        let svg = r#"<svg><rect id="test" /></svg>"#;
        let mut root = Element::parse(svg.as_bytes()).unwrap();
        let mut ops = HashMap::new();
        ops.insert("test".to_string(), vec![SvgOp::SetVisible(false), SvgOp::SetOpacity(0.5)]);
        apply_ops_to_svg(&mut root, ops);
        let el = find_element_by_id(&mut root, "test").unwrap();
        assert_eq!(el.attributes.get("display").unwrap(), "none");
        assert_eq!(el.attributes.get("opacity").unwrap(), "0.5");
    }

    #[test]
    fn test_append_element_and_clear() {
        let svg = r#"<svg><g id="container"></g></svg>"#;
        let mut root = Element::parse(svg.as_bytes()).unwrap();
        let mut ops = HashMap::new();
        let mut attrs = HashMap::new();
        attrs.insert("id".to_string(), "child".to_string());
        ops.insert("container".to_string(), vec![SvgOp::AppendElement { tag: "circle".to_string(), attributes: attrs }]);
        apply_ops_to_svg(&mut root, ops);
        assert!(find_element_by_id(&mut root, "child").is_some());

        let mut ops2 = HashMap::new();
        ops2.insert("container".to_string(), vec![SvgOp::ClearChildren]);
        apply_ops_to_svg(&mut root, ops2);
        assert!(find_element_by_id(&mut root, "child").is_none());
    }

    #[test]
    fn test_remove_element() {
        let svg = r#"<svg><rect id="to-remove" /></svg>"#;
        let mut root = Element::parse(svg.as_bytes()).unwrap();
        let mut ops = HashMap::new();
        ops.insert("to-remove".to_string(), vec![SvgOp::Remove]);
        apply_ops_to_svg(&mut root, ops);
        assert!(find_element_by_id(&mut root, "to-remove").is_none());
    }

    #[test]
    fn test_combined_transforms() {
        let svg = r#"<svg><rect id="test" /></svg>"#;
        let mut root = Element::parse(svg.as_bytes()).unwrap();
        let mut ops = HashMap::new();
        ops.insert("test".to_string(), vec![
            SvgOp::SetRotation { angle: 45.0, cx: 10.0, cy: 10.0 },
            SvgOp::SetTranslation { x: 5.0, y: 5.0 },
            SvgOp::SetScale { factor: 2.0 }
        ]);
        apply_ops_to_svg(&mut root, ops);
        let el = find_element_by_id(&mut root, "test").unwrap();
        let transform = el.attributes.get("transform").unwrap();
        assert!(transform.contains("rotate(45, 10, 10)"));
        assert!(transform.contains("translate(5, 5)"));
        assert!(transform.contains("scale(2)"));
    }

    #[test]
    fn test_set_attribute() {
        let svg = r#"<svg><rect id="test" fill="red" /></svg>"#;
        let mut root = Element::parse(svg.as_bytes()).unwrap();
        let mut ops = HashMap::new();
        ops.insert("test".to_string(), vec![SvgOp::SetAttribute { name: "fill".to_string(), value: "blue".to_string() }]);
        apply_ops_to_svg(&mut root, ops);
        let el = find_element_by_id(&mut root, "test").unwrap();
        assert_eq!(el.attributes.get("fill").unwrap(), "blue");
    }

    #[test]
    fn test_full_js_integration() {
        let svg = r#"<svg viewBox="0 0 100 100"><rect id="rect1" x="0" y="0" width="10" height="10" /><g id="group1"></g></svg>"#;
        let mut root = Element::parse(svg.as_bytes()).unwrap();
        let mut js_context = JsContext::default();
        
        js_context.register_global_class::<WidgetAPI>().unwrap();
        js_context.register_global_class::<ElementHandle>().unwrap();
        js_context.register_global_class::<WidgetState>().unwrap();
        js_context.register_global_class::<crate::RefreshRequest>().unwrap();
        
        let api_proto = get_proto::<WidgetAPI>(&mut js_context);
        let state_proto = get_proto::<WidgetState>(&mut js_context);
        let request_proto = get_proto::<crate::RefreshRequest>(&mut js_context);
        
        let shared_ops = Arc::new(Mutex::new(HashMap::new()));
        let shared_state = Arc::new(Mutex::new(HashMap::new()));
        let refresh_delay = Arc::new(Mutex::new(None));
        let capture_keyboard = Arc::new(Mutex::new(false));
        let capture_clicks = Arc::new(Mutex::new(false));

        let js_code = r#"
            function update(api, timestamp, click, keys, state, request) {
                api.findById("rect1").setRotation(90).setOpacity(0.7);
                api.findById("group1").appendElement("circle", { id: "dynamic_circle", r: "5" });
                state.set("last_ts", timestamp.toString());
                request.refreshInMS(500);
                if (keys && keys[0] === "Enter") {
                    state.set("key_pressed", "true");
                }
            }
        "#;
        js_context.eval(Source::from_bytes(js_code.as_bytes())).unwrap();

        let api_data = WidgetAPI { ops: shared_ops.clone(), handle_proto: get_proto::<ElementHandle>(&mut js_context) };
        let js_api = JsObject::from_proto_and_data(Some(api_proto), api_data);
        
        let state_data = WidgetState { data: shared_state.clone(), states_file: PathBuf::from("test_states.yml") };
        let js_state = JsObject::from_proto_and_data(Some(state_proto), state_data);

        let request_data = crate::RefreshRequest { 
            delay_ms: refresh_delay.clone(),
            capture_keyboard: capture_keyboard.clone(),
            capture_clicks: capture_clicks.clone(),
            incoming_messages: Arc::new(Mutex::new(false)),
            exit_trigger: Arc::new(Mutex::new(None)),
            http_queue: Arc::new(Mutex::new(Vec::new())),
            cli_queue: Arc::new(Mutex::new(Vec::new())),
            outgoing_messages: Arc::new(Mutex::new(Vec::new())),
        };
        let js_request = JsObject::from_proto_and_data(Some(request_proto), request_data);

        let js_keys = boa_engine::object::builtins::JsArray::new(&mut js_context);
        js_keys.push(JsString::from("Enter"), &mut js_context).ok();

        let update_func = js_context.global_object().get(JsString::from("update"), &mut js_context).unwrap();
        update_func.as_object().unwrap().call(
            &JsValue::undefined(),
            &[js_api.into(), JsValue::new(12345), JsValue::undefined(), js_keys.into(), js_state.into(), js_request.into()],
            &mut js_context
        ).unwrap();

        let ops = shared_ops.lock().unwrap().clone();
        apply_ops_to_svg(&mut root, ops);

        let rect = find_element_by_id(&mut root, "rect1").unwrap();
        assert_eq!(rect.attributes.get("transform").unwrap(), "rotate(90, 50, 50)");
        assert_eq!(rect.attributes.get("opacity").unwrap(), "0.7");

        assert!(find_element_by_id(&mut root, "dynamic_circle").is_some());
        assert_eq!(shared_state.lock().unwrap().get("last_ts").unwrap(), "12345");
        assert_eq!(shared_state.lock().unwrap().get("key_pressed").unwrap(), "true");
        assert_eq!(refresh_delay.lock().unwrap().unwrap(), 500);
    }
}
