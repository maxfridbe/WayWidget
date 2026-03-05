
#[cfg(test)]
mod tests {
    use crate::{apply_ops_to_svg, find_element_by_id, SvgOp, WidgetAPI, WidgetState, ElementHandle, JsContext, get_proto};
    use xmltree::Element;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use boa_engine::{Source, JsValue, JsString, JsObject};

    fn setup_svg() -> Element {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <rect id="rect1" x="10" y="10" width="20" height="20" fill="red" />
            <circle id="circle1" cx="50" cy="50" r="10" fill="blue" />
            <g id="group1">
                <path id="path1" d="M 0 0 L 10 10" />
            </g>
        </svg>"#;
        Element::parse(svg.as_bytes()).unwrap()
    }

    #[test]
    fn test_set_attribute() {
        let mut root = setup_svg();
        let mut ops = HashMap::new();
        ops.insert("rect1".to_string(), vec![SvgOp::SetAttribute { name: "fill".to_string(), value: "green".to_string() }]);
        
        apply_ops_to_svg(&mut root, ops);
        
        let rect = find_element_by_id(&mut root, "rect1").unwrap();
        assert_eq!(rect.attributes.get("fill").unwrap(), "green");
    }

    #[test]
    fn test_combined_transforms() {
        let mut root = setup_svg();
        let mut ops = HashMap::new();
        ops.insert("circle1".to_string(), vec![
            SvgOp::SetRotation { angle: 45.0, cx: 50.0, cy: 50.0 },
            SvgOp::SetTranslation { x: 10.0, y: 20.0 },
            SvgOp::SetScale { factor: 2.0 },
        ]);
        
        apply_ops_to_svg(&mut root, ops);
        
        let circle = find_element_by_id(&mut root, "circle1").unwrap();
        let transform = circle.attributes.get("transform").unwrap();
        assert!(transform.contains("rotate(45, 50, 50)"));
        assert!(transform.contains("translate(10, 20)"));
        assert!(transform.contains("scale(2)"));
    }

    #[test]
    fn test_visibility_and_opacity() {
        let mut root = setup_svg();
        let mut ops = HashMap::new();
        ops.insert("rect1".to_string(), vec![
            SvgOp::SetVisible(false),
            SvgOp::SetOpacity(0.5),
        ]);
        
        apply_ops_to_svg(&mut root, ops);
        
        let rect = find_element_by_id(&mut root, "rect1").unwrap();
        assert_eq!(rect.attributes.get("display").unwrap(), "none");
        assert_eq!(rect.attributes.get("opacity").unwrap(), "0.5");

        let mut ops2 = HashMap::new();
        ops2.insert("rect1".to_string(), vec![SvgOp::SetVisible(true)]);
        apply_ops_to_svg(&mut root, ops2);
        let rect2 = find_element_by_id(&mut root, "rect1").unwrap();
        assert!(rect2.attributes.get("display").is_none());
    }

    #[test]
    fn test_class_management() {
        let mut root = setup_svg();
        let mut ops = HashMap::new();
        ops.insert("group1".to_string(), vec![
            SvgOp::AddClass("active".to_string()),
            SvgOp::AddClass("highlight".to_string()),
        ]);
        
        apply_ops_to_svg(&mut root, ops);
        
        let group = find_element_by_id(&mut root, "group1").unwrap();
        let classes = group.attributes.get("class").unwrap();
        assert!(classes.contains("active"));
        assert!(classes.contains("highlight"));

        let mut ops2 = HashMap::new();
        ops2.insert("group1".to_string(), vec![SvgOp::RemoveClass("active".to_string())]);
        apply_ops_to_svg(&mut root, ops2);
        let group2 = find_element_by_id(&mut root, "group1").unwrap();
        let classes2 = group2.attributes.get("class").unwrap();
        assert!(!classes2.contains("active"));
        assert!(classes2.contains("highlight"));
    }

    #[test]
    fn test_append_element_and_clear() {
        let mut root = setup_svg();
        let mut ops = HashMap::new();
        let mut attrs = HashMap::new();
        attrs.insert("id".to_string(), "new_rect".to_string());
        attrs.insert("width".to_string(), "5".to_string());

        ops.insert("group1".to_string(), vec![
            SvgOp::AppendElement { tag: "rect".to_string(), attributes: attrs },
        ]);
        
        apply_ops_to_svg(&mut root, ops);
        
        assert!(find_element_by_id(&mut root, "new_rect").is_some());

        let mut ops2 = HashMap::new();
        ops2.insert("group1".to_string(), vec![SvgOp::ClearChildren]);
        apply_ops_to_svg(&mut root, ops2);
        
        let group = find_element_by_id(&mut root, "group1").unwrap();
        assert!(group.children.is_empty());
    }

    #[test]
    fn test_remove_element() {
        let mut root = setup_svg();
        let mut ops = HashMap::new();
        ops.insert("circle1".to_string(), vec![SvgOp::Remove]);
        
        apply_ops_to_svg(&mut root, ops);
        
        assert!(find_element_by_id(&mut root, "circle1").is_none());
    }

    #[test]
    fn test_full_js_integration() {
        let mut root = setup_svg();
        let mut js_context = JsContext::default();
        
        js_context.register_global_class::<WidgetAPI>().unwrap();
        js_context.register_global_class::<ElementHandle>().unwrap();
        js_context.register_global_class::<WidgetState>().unwrap();
        
        let api_proto = get_proto::<WidgetAPI>(&mut js_context);
        let state_proto = get_proto::<WidgetState>(&mut js_context);
        
        let shared_ops = Arc::new(Mutex::new(HashMap::new()));
        let shared_state = Arc::new(Mutex::new(HashMap::new()));

        let js_code = r#"
            function update(api, timestamp, click, state) {
                api.findById("rect1").setRotation(90).setOpacity(0.7);
                api.findById("group1").appendElement("circle", { id: "dynamic_circle", r: "5" });
                state.set("last_ts", timestamp.toString());
            }
        "#;
        js_context.eval(Source::from_bytes(js_code.as_bytes())).unwrap();

        let api_data = WidgetAPI { ops: shared_ops.clone(), handle_proto: get_proto::<ElementHandle>(&mut js_context) };
        let js_api = JsObject::from_proto_and_data(Some(api_proto), api_data);
        
        let state_data = WidgetState { data: shared_state.clone() };
        let js_state = JsObject::from_proto_and_data(Some(state_proto), state_data);

        let update_func = js_context.global_object().get(JsString::from("update"), &mut js_context).unwrap();
        update_func.as_object().unwrap().call(
            &JsValue::undefined(),
            &[js_api.into(), JsValue::new(12345), JsValue::undefined(), js_state.into()],
            &mut js_context
        ).unwrap();

        let ops = shared_ops.lock().unwrap().clone();
        apply_ops_to_svg(&mut root, ops);

        let rect = find_element_by_id(&mut root, "rect1").unwrap();
        assert_eq!(rect.attributes.get("transform").unwrap(), "rotate(90, 50, 50)");
        assert_eq!(rect.attributes.get("opacity").unwrap(), "0.7");

        assert!(find_element_by_id(&mut root, "dynamic_circle").is_some());
        assert_eq!(shared_state.lock().unwrap().get("last_ts").unwrap(), "12345");
    }
}
