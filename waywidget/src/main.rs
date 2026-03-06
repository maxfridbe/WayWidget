use std::time::Duration;
use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use clap::Parser;

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_output, delegate_pointer, delegate_registry, delegate_seat,
    delegate_shm, delegate_xdg_shell, delegate_xdg_window,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    seat::{
        pointer::{PointerEvent, PointerEventKind, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shm::{slot::SlotPool, Shm, ShmHandler},
    shell::xdg::{
        window::{Window, WindowConfigure, WindowHandler, WindowDecorations},
        XdgShell,
    },
    shell::WaylandSurface,
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_pointer, wl_seat, wl_shm, wl_surface, wl_output},
    Connection, QueueHandle,
};
use wayland_protocols::xdg::shell::client::xdg_toplevel::ResizeEdge;

use cairo::{Context as CairoContext, ImageSurface, Format};
use rsvg::{Loader, CairoRenderer};
use xmltree::Element;
use gio::MemoryInputStream;

use calloop::{EventLoop, timer::{Timer, TimeoutAction}};
use calloop_wayland_source::WaylandSource;

use boa_engine::{
    Context as JsContext, Source, JsValue, JsString, JsObject, JsResult, JsError, JsArgs,
    class::{Class, ClassBuilder}, 
    native_function::NativeFunction,
    JsData,
};
use boa_gc::{Finalize, Trace};

use directories::ProjectDirs;
use serde::{Serialize, Deserialize};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long)]
    svg: Option<PathBuf>,
    #[arg(short = 'j', long)]
    script: Option<PathBuf>,
    #[arg(long, default_value_t = 200)]
    width: u32,
    #[arg(long, default_value_t = 200)]
    height: u32,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    Run {
        widget: String,
        #[arg(short, long)]
        name: Option<String>,
        #[arg(long)]
        width: Option<u32>,
        #[arg(long)]
        height: Option<u32>,
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
struct WidgetConfig {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct Positions {
    #[serde(flatten)]
    widgets: HashMap<String, WidgetConfig>,
}

#[derive(Debug, Clone)]
enum SvgOp {
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

#[derive(Clone, Trace, Finalize, JsData)]
struct RefreshRequest {
    #[unsafe_ignore_trace]
    delay_ms: Arc<Mutex<Option<u32>>>,
}

impl Class for RefreshRequest {
    const NAME: &'static str = "RefreshRequest";
    fn data_constructor(_this: &JsValue, _args: &[JsValue], _context: &mut JsContext) -> JsResult<Self> {
        Err(JsError::from_opaque(JsString::from("Cannot construct RefreshRequest directly").into()))
    }
    fn init(class: &mut ClassBuilder<'_>) -> JsResult<()> {
        class.method(JsString::from("refreshInMS"), 1, NativeFunction::from_fn_ptr(|this, args, _context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let request = obj.downcast_ref::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not a RefreshRequest").into()))?;
            let ms = args.get_or_undefined(0).as_number().unwrap_or(0.0) as u32;
            let mut delay = request.delay_ms.lock().unwrap();
            *delay = Some(ms.max(33));
            Ok(JsValue::undefined())
        }));
        Ok(())
    }
}

#[derive(Clone, Trace, Finalize, JsData)]
struct ElementHandle {
    id: String,
    #[unsafe_ignore_trace]
    ops: Arc<Mutex<HashMap<String, Vec<SvgOp>>>>,
}

impl Class for ElementHandle {
    const NAME: &'static str = "ElementHandle";
    fn data_constructor(_this: &JsValue, _args: &[JsValue], _context: &mut JsContext) -> JsResult<Self> {
        Err(JsError::from_opaque(JsString::from("Cannot construct ElementHandle directly").into()))
    }
    fn init(class: &mut ClassBuilder<'_>) -> JsResult<()> {
        class.method(JsString::from("setRotation"), 3, NativeFunction::from_fn_ptr(|this, args, _context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let handle = obj.downcast_mut::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not an ElementHandle").into()))?;
            let angle = args.get_or_undefined(0).as_number().unwrap_or(0.0);
            let cx = args.get_or_undefined(1).as_number().unwrap_or(50.0);
            let cy = args.get_or_undefined(2).as_number().unwrap_or(50.0);
            handle.ops.lock().unwrap().entry(handle.id.clone()).or_default().push(SvgOp::SetRotation { angle, cx, cy });
            Ok(this.clone())
        }));
        class.method(JsString::from("setTranslation"), 2, NativeFunction::from_fn_ptr(|this, args, _context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let handle = obj.downcast_mut::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not an ElementHandle").into()))?;
            let x = args.get_or_undefined(0).as_number().unwrap_or(0.0);
            let y = args.get_or_undefined(1).as_number().unwrap_or(0.0);
            handle.ops.lock().unwrap().entry(handle.id.clone()).or_default().push(SvgOp::SetTranslation { x, y });
            Ok(this.clone())
        }));
        class.method(JsString::from("setScale"), 1, NativeFunction::from_fn_ptr(|this, args, _context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let handle = obj.downcast_mut::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not an ElementHandle").into()))?;
            let factor = args.get_or_undefined(0).as_number().unwrap_or(1.0);
            handle.ops.lock().unwrap().entry(handle.id.clone()).or_default().push(SvgOp::SetScale { factor });
            Ok(this.clone())
        }));
        class.method(JsString::from("setText"), 1, NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let handle = obj.downcast_mut::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not an ElementHandle").into()))?;
            let text = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();
            handle.ops.lock().unwrap().entry(handle.id.clone()).or_default().push(SvgOp::SetText(text));
            Ok(this.clone())
        }));
        class.method(JsString::from("setAttribute"), 2, NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let handle = obj.downcast_mut::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not an ElementHandle").into()))?;
            let name = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();
            let value = args.get_or_undefined(1).to_string(context)?.to_std_string().unwrap();
            handle.ops.lock().unwrap().entry(handle.id.clone()).or_default().push(SvgOp::SetAttribute { name, value });
            Ok(this.clone())
        }));
        class.method(JsString::from("setVisible"), 1, NativeFunction::from_fn_ptr(|this, args, _context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let handle = obj.downcast_mut::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not an ElementHandle").into()))?;
            let visible = args.get_or_undefined(0).as_boolean().unwrap_or(true);
            handle.ops.lock().unwrap().entry(handle.id.clone()).or_default().push(SvgOp::SetVisible(visible));
            Ok(this.clone())
        }));
        class.method(JsString::from("setOpacity"), 1, NativeFunction::from_fn_ptr(|this, args, _context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let handle = obj.downcast_mut::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not an ElementHandle").into()))?;
            let opacity = args.get_or_undefined(0).as_number().unwrap_or(1.0);
            handle.ops.lock().unwrap().entry(handle.id.clone()).or_default().push(SvgOp::SetOpacity(opacity));
            Ok(this.clone())
        }));
        class.method(JsString::from("addClass"), 1, NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let handle = obj.downcast_mut::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not an ElementHandle").into()))?;
            let class_name = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();
            handle.ops.lock().unwrap().entry(handle.id.clone()).or_default().push(SvgOp::AddClass(class_name));
            Ok(this.clone())
        }));
        class.method(JsString::from("removeClass"), 1, NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let handle = obj.downcast_mut::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not an ElementHandle").into()))?;
            let class_name = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();
            handle.ops.lock().unwrap().entry(handle.id.clone()).or_default().push(SvgOp::RemoveClass(class_name));
            Ok(this.clone())
        }));
        class.method(JsString::from("appendElement"), 2, NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let handle = obj.downcast_mut::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not an ElementHandle").into()))?;
            let tag = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();
            let attr_obj = args.get_or_undefined(1).as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Attributes must be an object").into()))?;
            let mut attributes = HashMap::new();
            let keys = attr_obj.own_property_keys(context)?;
            for key in keys {
                let key_js = JsValue::from(key.clone());
                let key_str = key_js.to_string(context)?.to_std_string().unwrap();
                let val_str = attr_obj.get(key, context)?.to_string(context)?.to_std_string().unwrap();
                attributes.insert(key_str, val_str);
            }
            handle.ops.lock().unwrap().entry(handle.id.clone()).or_default().push(SvgOp::AppendElement { tag, attributes });
            Ok(this.clone())
        }));
        class.method(JsString::from("clearChildren"), 0, NativeFunction::from_fn_ptr(|this, _args, _context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let handle = obj.downcast_mut::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not an ElementHandle").into()))?;
            handle.ops.lock().unwrap().entry(handle.id.clone()).or_default().push(SvgOp::ClearChildren);
            Ok(this.clone())
        }));
        class.method(JsString::from("remove"), 0, NativeFunction::from_fn_ptr(|this, _args, _context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let handle = obj.downcast_mut::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not an ElementHandle").into()))?;
            handle.ops.lock().unwrap().entry(handle.id.clone()).or_default().push(SvgOp::Remove);
            Ok(this.clone())
        }));
        Ok(())
    }
}

#[derive(Clone, Trace, Finalize, JsData)]
struct WidgetAPI {
    #[unsafe_ignore_trace]
    ops: Arc<Mutex<HashMap<String, Vec<SvgOp>>>>,
    #[unsafe_ignore_trace]
    handle_proto: JsObject,
}

impl Class for WidgetAPI {
    const NAME: &'static str = "WidgetAPI";
    fn data_constructor(_this: &JsValue, _args: &[JsValue], _context: &mut JsContext) -> JsResult<Self> {
        Err(JsError::from_opaque(JsString::from("Cannot construct WidgetAPI directly").into()))
    }
    fn init(class: &mut ClassBuilder<'_>) -> JsResult<()> {
        class.method(JsString::from("findById"), 1, NativeFunction::from_fn_ptr(|this, args, context| {
            let id = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let api = obj.downcast_ref::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not a WidgetAPI").into()))?;
            let handle = ElementHandle { id, ops: api.ops.clone() };
            Ok(JsObject::from_proto_and_data(Some(api.handle_proto.clone()), handle).into())
        }));
        Ok(())
    }
}

#[derive(Clone, Trace, Finalize, JsData)]
struct WidgetState {
    #[unsafe_ignore_trace]
    data: Arc<Mutex<HashMap<String, String>>>,
}

impl Class for WidgetState {
    const NAME: &'static str = "WidgetState";
    fn data_constructor(_this: &JsValue, _args: &[JsValue], _context: &mut JsContext) -> JsResult<Self> {
        Err(JsError::from_opaque(JsString::from("Cannot construct WidgetState directly").into()))
    }
    fn init(class: &mut ClassBuilder<'_>) -> JsResult<()> {
        class.method(JsString::from("set"), 2, NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let state = obj.downcast_ref::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not a WidgetState").into()))?;
            let key = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();
            let val = args.get_or_undefined(1).to_string(context)?.to_std_string().unwrap();
            
            let mut data = state.data.lock().unwrap();
            let old_val = data.get(&key);
            if old_val != Some(&val) {
                println!("State Set: {} = {}", key, val);
                data.insert(key, val);
            }
            Ok(JsValue::undefined())
        }));
        class.method(JsString::from("clear"), 1, NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let state = obj.downcast_ref::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not a WidgetState").into()))?;
            let key = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();
            state.data.lock().unwrap().remove(&key);
            Ok(JsValue::undefined())
        }));
        class.method(JsString::from("setObject"), 2, NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let state = obj.downcast_ref::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not a WidgetState").into()))?;
            let key = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();
            let val = args.get_or_undefined(1);
            
            let json = context.global_object().get(JsString::from("JSON"), context)?.as_object().expect("JSON global exists").clone();
            let stringify = json.get(JsString::from("stringify"), context)?.as_object().expect("JSON.stringify exists").clone();
            let stringified = stringify.call(&json.into(), &[val.clone()], context)?.to_string(context)?.to_std_string().unwrap();

            let mut data = state.data.lock().unwrap();
            let old_val = data.get(&key);
            if old_val != Some(&stringified) {
                println!("State Set Object: {} = {}", key, stringified);
                data.insert(key, stringified);
            }
            Ok(JsValue::undefined())
        }));
        class.method(JsString::from("getObject"), 1, NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let state = obj.downcast_ref::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not a WidgetState").into()))?;
            let key = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();
            let val = state.data.lock().unwrap().get(&key).cloned().unwrap_or_default();
            if val.is_empty() {
                return Ok(JsValue::null());
            }
            
            let json = context.global_object().get(JsString::from("JSON"), context)?.as_object().expect("JSON global exists").clone();
            let parse = json.get(JsString::from("parse"), context)?.as_object().expect("JSON.parse exists").clone();
            parse.call(&json.into(), &[JsString::from(val).into()], context)
        }));
        class.method(JsString::from("get"), 1, NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let state = obj.downcast_ref::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not a WidgetState").into()))?;
            let key = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();
            let val = state.data.lock().unwrap().get(&key).cloned().unwrap_or_default();
            Ok(JsString::from(val).into())
        }));
        Ok(())
    }
}

struct WayWidget {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    _compositor_state: CompositorState,
    _shm_state: Shm,
    _xdg_shell_state: XdgShell,

    window: Window,
    pool: SlotPool,
    qh: QueueHandle<Self>,
    
    svg_root: Element,
    viewbox: (f64, f64),
    svg_handle: Option<rsvg::SvgHandle>,
    
    js_context: JsContext,
    api_proto: JsObject,
    handle_proto: JsObject,
    state_proto: JsObject,
    request_proto: JsObject,
    shared_ops: Arc<Mutex<HashMap<String, Vec<SvgOp>>>>,
    shared_state: Arc<Mutex<HashMap<String, String>>>,
    refresh_delay: Arc<Mutex<Option<u32>>>,
    
    pointer: Option<wl_pointer::WlPointer>,
    seat: Option<wl_seat::WlSeat>,
    pointer_pos: (f64, f64),
    last_click: Option<(f64, f64)>,
    is_hovering: bool,
    
    exit: bool,
    width: u32,
    height: u32,
    needs_redraw: bool,
    
    widget_name: String,
    positions_file: PathBuf,
    current_config: WidgetConfig,
}

pub(crate) fn find_element_by_id<'a>(el: &'a mut Element, id: &str) -> Option<&'a mut Element> {
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

pub(crate) fn remove_element_by_id(el: &mut Element, id: &str) -> bool {
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

pub(crate) fn apply_ops_to_svg(root: &mut Element, ops: HashMap<String, Vec<SvgOp>>) {
    for (id, el_ops) in ops {
        let mut should_remove = false;
        for op in &el_ops {
            if let SvgOp::Remove = op {
                should_remove = true;
                break;
            }
        }

        if should_remove {
            remove_element_by_id(root, &id);
            continue;
        }

        if let Some(el) = find_element_by_id(root, &id) {
            let mut transforms = Vec::new();
            for op in el_ops {
                match op {
                    SvgOp::SetRotation { angle, cx, cy } => {
                        transforms.push(format!("rotate({}, {}, {})", angle, cx, cy));
                    }
                    SvgOp::SetTranslation { x, y } => {
                        transforms.push(format!("translate({}, {})", x, y));
                    }
                    SvgOp::SetScale { factor } => {
                        transforms.push(format!("scale({})", factor));
                    }
                    SvgOp::SetText(text) => {
                        el.children.clear();
                        el.children.push(xmltree::XMLNode::Text(text));
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
                        let mut child = xmltree::Element::new(&tag);
                        child.attributes = attributes;
                        el.children.push(xmltree::XMLNode::Element(child));
                    }                    SvgOp::ClearChildren => {
                        el.children.clear();
                    }
                    SvgOp::Remove => {} // Handled above
                }
            }
            if !transforms.is_empty() {
                el.attributes.insert("transform".to_string(), transforms.join(" "));
            }
        }
    }
}

impl WayWidget {
    fn save_positions(&self) {
        let mut positions: Positions = if self.positions_file.exists() {
            let f = fs::File::open(&self.positions_file).unwrap();
            serde_yaml::from_reader(f).unwrap_or_default()
        } else {
            Positions::default()
        };
        let mut cfg = self.current_config.clone();
        cfg.width = self.width;
        cfg.height = self.height;
        positions.widgets.insert(self.widget_name.clone(), cfg);
        if let Ok(f) = fs::File::create(&self.positions_file) {
            serde_yaml::to_writer(f, &positions).ok();
        }
    }

    fn draw(&mut self) {
        // 1. Get JS updates
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as f64;
        self.shared_ops.lock().unwrap().clear();
        
        let update_name = JsString::from("update");
        let global = self.js_context.global_object();
        if global.has_property(update_name.clone(), &mut self.js_context).unwrap_or(false) {
            let update_func = global.get(update_name, &mut self.js_context).unwrap();
            if let Some(func) = update_func.as_object() {
                let api_data = WidgetAPI { ops: self.shared_ops.clone(), handle_proto: self.handle_proto.clone() };
                let js_api = JsObject::from_proto_and_data(Some(self.api_proto.clone()), api_data);

                let state_data = WidgetState { data: self.shared_state.clone() };
                let js_state = JsObject::from_proto_and_data(Some(self.state_proto.clone()), state_data);

                let request_data = RefreshRequest { delay_ms: self.refresh_delay.clone() };
                let js_request = JsObject::from_proto_and_data(Some(self.request_proto.clone()), request_data);

                let click_val = if let Some((x, y)) = self.last_click.take() {
                    let obj = JsObject::default(self.js_context.intrinsics());
                    obj.set(JsString::from("x"), JsValue::new(x), true, &mut self.js_context).ok();
                    obj.set(JsString::from("y"), JsValue::new(y), true, &mut self.js_context).ok();
                    obj.into()
                } else {
                    JsValue::undefined()
                };
                
                func.call(&JsValue::undefined(), &[js_api.into(), JsValue::new(timestamp), click_val, js_state.into(), js_request.into()], &mut self.js_context)
                    .map_err(|e| println!("JS Error in update(): {}", e))
                    .ok();
            }
        }
        
        let ops = self.shared_ops.lock().unwrap().clone();
        let has_ops = !ops.is_empty();

        // 2. Apply to tree if needed
        if has_ops {
            apply_ops_to_svg(&mut self.svg_root, ops);
            
            // Re-parse tree
            let mut out = Vec::new();
            self.svg_root.write(&mut out).ok();
            let bytes = glib::Bytes::from(&out);
            let stream = MemoryInputStream::from_bytes(&bytes);
            self.svg_handle = Some(Loader::new().read_stream(&stream, None as Option<&gio::File>, None as Option<&gio::Cancellable>).expect("load svg data"));
            
            self.needs_redraw = true;
        }

        if !self.needs_redraw || self.svg_handle.is_none() {
            return;
        }

        // 3. Zero-Copy Drawing
        let (buffer, canvas) = self
            .pool
            .create_buffer(
                self.width as i32,
                self.height as i32,
                self.width as i32 * 4,
                wl_shm::Format::Argb8888,
            )
            .expect("create buffer");

        unsafe {
            let canvas_static: &'static mut [u8] = std::mem::transmute(canvas);
            let surface = ImageSurface::create_for_data(
                canvas_static,
                Format::ARgb32,
                self.width as i32,
                self.height as i32,
                self.width as i32 * 4,
            ).expect("cairo surface");
            
            let cr = CairoContext::new(&surface).expect("cairo context");
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
            cr.set_operator(cairo::Operator::Source);
            cr.paint().expect("paint clear");
            cr.set_operator(cairo::Operator::Over);

            let renderer = CairoRenderer::new(self.svg_handle.as_ref().unwrap());
            
            cr.save().expect("save content");
            let (vb_w, vb_h) = self.viewbox;
            cr.scale(self.width as f64 / vb_w, self.height as f64 / vb_h);
            renderer.render_document(&cr, &cairo::Rectangle::new(0.0, 0.0, vb_w, vb_h)).ok();
            cr.restore().expect("restore content");

            if self.is_hovering {
                cr.set_source_rgba(1.0, 1.0, 1.0, 0.3);
                let w = self.width as f64;
                let h = self.height as f64;
                cr.move_to(w, h - 20.0);
                cr.line_to(w, h);
                cr.line_to(w - 20.0, h);
                cr.close_path();
                cr.fill().expect("fill handle");
            }
            surface.flush();
        }

        self.window.wl_surface().attach(Some(buffer.wl_buffer()), 0, 0);
        self.window.wl_surface().damage_buffer(0, 0, self.width as i32, self.height as i32);
        self.window.wl_surface().commit();
        self.needs_redraw = false;
    }
}

impl CompositorHandler for WayWidget {
    fn scale_factor_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _new_factor: i32) {}
    fn transform_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _new_transform: wl_output::Transform) {}
    fn frame(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _time: u32) {
        if self.needs_redraw { self.draw(); }
    }
    fn surface_enter(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _output: &wl_output::WlOutput) {}
    fn surface_leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _output: &wl_output::WlOutput) {}
}

impl OutputHandler for WayWidget {
    fn output_state(&mut self) -> &mut OutputState { &mut self.output_state }
    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
}

impl ShmHandler for WayWidget {
    fn shm_state(&mut self) -> &mut Shm { &mut self._shm_state }
}

impl WindowHandler for WayWidget {
    fn request_close(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _window: &Window) { self.exit = true; }
    fn configure(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _window: &Window, configure: WindowConfigure, _serial: u32) {
        let (w, h) = configure.new_size;
        let new_w = w.map(|v| v.get()).unwrap_or(self.width);
        let new_h = h.map(|v| v.get()).unwrap_or(self.height);
        if new_w != self.width || new_h != self.height {
            self.width = new_w;
            self.height = new_h;
            self.save_positions();
        }
        self.needs_redraw = true;
        self.draw();
    }
}

impl SeatHandler for WayWidget {
    fn seat_state(&mut self) -> &mut SeatState { &mut self.seat_state }
    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, seat: wl_seat::WlSeat) { self.seat = Some(seat); }
    fn new_capability(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat, capability: Capability) {
        if self.seat.is_none() { self.seat = Some(seat.clone()); }
        if capability == Capability::Pointer && self.pointer.is_none() {
            let pointer = self.seat_state.get_pointer(qh, &seat).expect("get pointer");
            self.pointer = Some(pointer);
        }
    }
    fn remove_capability(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat, capability: Capability) {
        if capability == Capability::Pointer { self.pointer = None; }
    }
    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat) {
        self.seat = None;
    }
}

impl PointerHandler for WayWidget {
    fn pointer_frame(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _pointer: &wl_pointer::WlPointer, events: &[PointerEvent]) {
        for event in events {
            self.pointer_pos = event.position;
            match event.kind {
                PointerEventKind::Enter { .. } => { self.is_hovering = true; self.needs_redraw = true; }
                PointerEventKind::Leave { .. } => { self.is_hovering = false; self.needs_redraw = true; }
                PointerEventKind::Motion { .. } => {}
                PointerEventKind::Press { button, serial, .. } => {
                    if button == 0x110 {
                        let (px, py) = self.pointer_pos;
                        self.last_click = Some((px / self.width as f64, py / self.height as f64));
                        self.needs_redraw = true;
                        if let Some(seat) = &self.seat {
                            if px > self.width as f64 - 20.0 && py > self.height as f64 - 20.0 {
                                self.window.xdg_toplevel().resize(seat, serial, ResizeEdge::BottomRight);
                            } else {
                                self.window.xdg_toplevel()._move(seat, serial);
                            }
                        }
                        self.draw();
                    }
                }
                _ => {}
            }
        }
    }
}

delegate_compositor!(WayWidget);
delegate_output!(WayWidget);
delegate_shm!(WayWidget);
delegate_seat!(WayWidget);
delegate_pointer!(WayWidget);
delegate_xdg_shell!(WayWidget);
delegate_xdg_window!(WayWidget);
delegate_registry!(WayWidget);

impl ProvidesRegistryState for WayWidget {
    fn registry(&mut self) -> &mut RegistryState { &mut self.registry_state }
    smithay_client_toolkit::registry_handlers![SeatState, OutputState];
}

fn get_proto<T: Class>(js_context: &mut JsContext) -> JsObject {
    js_context.global_object()
        .get(JsString::from(T::NAME), js_context).unwrap()
        .as_object().unwrap()
        .get(JsString::from("prototype"), js_context).unwrap()
        .as_object().unwrap()
        .clone()
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    
    let proj_dirs = ProjectDirs::from("org", "waywidget", "waywidget")
        .ok_or_else(|| anyhow::anyhow!("Could not determine project directories"))?;
    let config_dir = proj_dirs.config_dir();
    fs::create_dir_all(config_dir).ok();

    let (svg_path, script_path, width, height, widget_name) = match &args.command {
        Some(Commands::Run { widget, name, width, height }) => {
            let widget_dir = config_dir.join(widget);
            let svg = widget_dir.join("widget.svg");
            let script = widget_dir.join("widget.js");
            let name = name.clone().unwrap_or_else(|| widget.clone());
            (svg, Some(script), width.unwrap_or(200), height.unwrap_or(200), name)
        }
        None => {
            let svg = args.svg.ok_or_else(|| anyhow::anyhow!("SVG path required if not using 'run'"))?;
            let name = svg.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
            (svg, args.script, args.width, args.height, name)
        }
    };

    let positions_file = config_dir.join("positions.yml");
    let mut positions: Positions = if positions_file.exists() {
        let f = fs::File::open(&positions_file)?;
        serde_yaml::from_reader(f).unwrap_or_default()
    } else {
        Positions::default()
    };

    let cfg = positions.widgets.get(&widget_name).cloned().unwrap_or_default();
    let final_width = if cfg.width > 0 { cfg.width } else { width };
    let final_height = if cfg.height > 0 { cfg.height } else { height };
    println!("Starting widget '{}' at position: {:?}, size: {}x{}", widget_name, cfg, final_width, final_height);

    let conn = Connection::connect_to_env().expect("connect to wayland");
    let (globals, event_queue) = registry_queue_init::<WayWidget>(&conn).expect("registry init");
    let qh = event_queue.handle();

    let registry_state = RegistryState::new(&globals);
    let seat_state = SeatState::new(&globals, &qh);
    let output_state = OutputState::new(&globals, &qh);
    let compositor_state = CompositorState::bind(&globals, &qh).expect("bind compositor");
    let shm_state = Shm::bind(&globals, &qh).expect("bind shm");
    let xdg_shell_state = XdgShell::bind(&globals, &qh).expect("bind xdg_shell");

    let surface = compositor_state.create_surface(&qh);
    let window = xdg_shell_state.create_window(surface, WindowDecorations::None, &qh);
    window.set_title("WayWidget");
    window.set_app_id("waywidget");
    window.set_min_size(Some((100, 100)));
    window.set_max_size(Some((1200, 1200)));
    window.commit();

    let pool = SlotPool::new(1200 * 1200 * 4 * 2, &shm_state).expect("create pool");
    let svg_template = fs::read_to_string(&svg_path).expect("read svg");
    let svg_root = Element::parse(svg_template.as_bytes()).expect("parse svg");
    let viewbox_str = svg_root.attributes.get("viewBox").cloned().unwrap_or("0 0 100 100".to_string());
    let parts: Vec<f64> = viewbox_str.split_whitespace().filter_map(|s| s.parse().ok()).collect();
    let viewbox = if parts.len() == 4 { (parts[2], parts[3]) } else { (100.0, 100.0) };

    let mut js_context = JsContext::default();
    
    // Console.log
    let log_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        for arg in args {
            print!("{} ", arg.to_string(context).unwrap().to_std_string().unwrap());
        }
        println!();
        Ok(JsValue::undefined())
    });
    let _log_val = js_context.register_global_builtin_callable(JsString::from("log_internal"), 0, log_fn).unwrap();
    js_context.eval(Source::from_bytes("var console = { log: log_internal };".as_bytes())).unwrap();

    js_context.register_global_class::<WidgetAPI>().unwrap();
    js_context.register_global_class::<ElementHandle>().unwrap();
    js_context.register_global_class::<WidgetState>().unwrap();
    js_context.register_global_class::<RefreshRequest>().unwrap();
    
    let api_proto = get_proto::<WidgetAPI>(&mut js_context);
    let handle_proto = get_proto::<ElementHandle>(&mut js_context);
    let state_proto = get_proto::<WidgetState>(&mut js_context);
    let request_proto = get_proto::<RefreshRequest>(&mut js_context);
    
    let shared_ops = Arc::new(Mutex::new(HashMap::new()));
    let shared_state = Arc::new(Mutex::new(HashMap::new()));
    let refresh_delay = Arc::new(Mutex::new(None));

    if let Some(path) = &script_path {
        let js_source = fs::read_to_string(path).expect("read script");
        js_context.eval(Source::from_bytes(js_source.as_bytes())).expect("eval script");
    }

    let mut app = WayWidget {
        registry_state, seat_state, output_state,
        _compositor_state: compositor_state, _shm_state: shm_state, _xdg_shell_state: xdg_shell_state,
        window, pool, qh: qh.clone(),
        svg_root, viewbox, svg_handle: None,
        js_context, api_proto, handle_proto, state_proto, request_proto, 
        shared_ops, shared_state, refresh_delay: refresh_delay.clone(),
        pointer: None, seat: None, pointer_pos: (0.0, 0.0), last_click: None, is_hovering: false,
        exit: false, width: final_width, height: final_height, needs_redraw: true,
        widget_name, positions_file: positions_file.clone(),
        current_config: cfg,
    };

    let mut event_loop: EventLoop<WayWidget> = EventLoop::try_new().expect("create event loop");
    let handle = event_loop.handle();
    WaylandSource::new(conn, event_queue).insert(handle.clone()).expect("insert wayland source");

    let timer = Timer::from_duration(Duration::from_millis(10));
    handle.insert_source(timer, move |_, _, app| {
        let delay = {
            let mut lock = app.refresh_delay.lock().unwrap();
            lock.take()
        };

        if let Some(ms) = delay {
            app.needs_redraw = true;
            let surface = app.window.wl_surface().clone();
            surface.frame(&app.qh, surface.clone());
            app.window.wl_surface().commit();
            TimeoutAction::ToDuration(Duration::from_millis(ms as u64))
        } else {
            if app.needs_redraw {
                app.draw();
            }
            TimeoutAction::ToDuration(Duration::from_millis(200))
        }
    }).expect("insert timer");

    while !app.exit {
        event_loop.dispatch(Duration::from_millis(10), &mut app).expect("dispatch");
    }

    Ok(())
}
mod tests;
