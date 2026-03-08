use std::collections::HashMap;
use xmltree::{Element, XMLNode};

#[derive(Debug, Clone)]
pub enum SvgOp {
    SetRotation { angle: f64, cx: f64, cy: f64 },
    SetTranslation { x: f64, y: f64 },
    SetScale { factor: f64 },
    SetText(String),
    SetAttribute { name: String, value: String },
    SetVisible(bool),
    AddClass(String),
    RemoveClass(String),
    SetOpacity(f64),
    AppendElement { tag: String, attributes: HashMap<String, String> },
    ClearChildren,
    Remove,
}

pub fn find_element_by_id<'a>(el: &'a mut Element, id: &str) -> Option<&'a mut Element> {
    if el.attributes.get("id").map(|s| s.as_str()) == Some(id) {
        return Some(el);
    }
    for child in &mut el.children {
        if let Some(e) = child.as_mut_element() {
            if let Some(found) = find_element_by_id(e, id) {
                return Some(found);
            }
        }
    }
    None
}

pub fn remove_element_by_id(el: &mut Element, id: &str) -> bool {
    let mut to_remove = None;
    for (i, child) in el.children.iter().enumerate() {
        if let Some(child_el) = child.as_element() {
            if child_el.attributes.get("id").map(|s| s.as_str()) == Some(id) {
                to_remove = Some(i);
                break;
            }
        }
    }
    if let Some(i) = to_remove {
        el.children.remove(i);
        return true;
    }
    for child in &mut el.children {
        if let Some(child_el) = child.as_mut_element() {
            if remove_element_by_id(child_el, id) {
                return true;
            }
        }
    }
    false
}

pub fn extract_all_ids(el: &Element, ids: &mut Vec<String>) {
    if let Some(id) = el.attributes.get("id") {
        ids.push(id.clone());
    }
    for child in &el.children {
        if let Some(child_el) = child.as_element() {
            extract_all_ids(child_el, ids);
        }
    }
}

pub fn apply_ops_to_svg(root: &mut Element, ops: Vec<(String, SvgOp)>) {
    for (id, op) in ops {
        if let SvgOp::Remove = op {
            remove_element_by_id(root, &id);
            continue;
        }

        if let Some(el) = find_element_by_id(root, &id) {
            match op {
                SvgOp::SetRotation { angle, cx, cy } => {
                    let mut transforms = parse_transforms(el.attributes.get("transform"));
                    transforms.retain(|t| !t.starts_with("rotate("));
                    transforms.push(format!("rotate({}, {}, {})", angle, cx, cy));
                    el.attributes.insert("transform".to_string(), transforms.join(" "));
                }
                SvgOp::SetTranslation { x, y } => {
                    let mut transforms = parse_transforms(el.attributes.get("transform"));
                    transforms.retain(|t| !t.starts_with("translate("));
                    transforms.push(format!("translate({}, {})", x, y));
                    el.attributes.insert("transform".to_string(), transforms.join(" "));
                }
                SvgOp::SetScale { factor } => {
                    let mut transforms = parse_transforms(el.attributes.get("transform"));
                    transforms.retain(|t| !t.starts_with("scale("));
                    transforms.push(format!("scale({})", factor));
                    el.attributes.insert("transform".to_string(), transforms.join(" "));
                }
                SvgOp::SetText(text) => {
                    el.children.clear();
                    el.children.push(XMLNode::Text(text));
                }
                SvgOp::SetAttribute { name, value } => {
                    el.attributes.insert(name, value);
                }
                SvgOp::SetVisible(visible) => {
                    if visible {
                        el.attributes.remove("display");
                    } else {
                        el.attributes.insert("display".to_string(), "none".to_string());
                    }
                }
                SvgOp::SetOpacity(opacity) => {
                    el.attributes.insert("opacity".to_string(), opacity.to_string());
                }
                SvgOp::AddClass(class_name) => {
                    let current = el.attributes.get("class").cloned().unwrap_or_default();
                    if !current.split_whitespace().any(|c| c == class_name) {
                        let new_class = if current.is_empty() { class_name } else { format!("{} {}", current, class_name) };
                        el.attributes.insert("class".to_string(), new_class);
                    }
                }
                SvgOp::RemoveClass(class_name) => {
                    if let Some(current) = el.attributes.get("class").cloned() {
                        let new_classes: Vec<&str> = current.split_whitespace().filter(|&c| c != class_name).collect();
                        el.attributes.insert("class".to_string(), new_classes.join(" "));
                    }
                }
                SvgOp::AppendElement { tag, attributes } => {
                    let mut child = Element::new(&tag);
                    child.attributes = attributes;
                    el.children.push(XMLNode::Element(child));
                }
                SvgOp::ClearChildren => {
                    el.children.clear();
                }
                SvgOp::Remove => {} // Handled above
            }
        }
    }
}

fn parse_transforms(transform_attr: Option<&String>) -> Vec<String> {
    match transform_attr {
        Some(s) => {
            // Very basic parser for multiple transforms
            let mut result = Vec::new();
            let mut current = String::new();
            let mut depth = 0;
            for c in s.chars() {
                current.push(c);
                if c == '(' { depth += 1; }
                else if c == ')' {
                    depth -= 1;
                    if depth == 0 {
                        result.push(current.trim().to_string());
                        current = String::new();
                    }
                }
            }
            if !current.trim().is_empty() {
                result.push(current.trim().to_string());
            }
            result
        }
        None => Vec::new(),
    }
}
