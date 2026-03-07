use std::time::Duration;
use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use clap::Parser;

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_output, delegate_pointer, delegate_registry, delegate_seat,
    delegate_shm, delegate_xdg_shell, delegate_xdg_window, delegate_keyboard,
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
    protocol::{wl_pointer, wl_seat, wl_shm, wl_surface, wl_output, wl_keyboard},
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

mod svg;
mod network;
mod cli;
mod keyboard;

use svg::{SvgOp, apply_ops_to_svg};
use network::{HttpMethod, HttpCall, HttpResult, process_http_queue};
use cli::{CliCall, CliResult, process_cli_queue};

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
    #[arg(long)]
    position: Option<String>,
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
        #[arg(long)]
        position: Option<String>,
    },
    Stop {
        #[arg(short, long)]
        name: String,
    }
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct WidgetConfig {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct Positions {
    #[serde(flatten)]
    widgets: HashMap<String, WidgetConfig>,
}

#[derive(Clone, Trace, Finalize, JsData)]
struct RefreshRequest {
    #[unsafe_ignore_trace]
    delay_ms: Arc<Mutex<Option<u32>>>,
    #[unsafe_ignore_trace]
    capture_keyboard: Arc<Mutex<bool>>,
    #[unsafe_ignore_trace]
    capture_clicks: Arc<Mutex<bool>>,
    #[unsafe_ignore_trace]
    http_queue: Arc<Mutex<Vec<HttpCall>>>,
    #[unsafe_ignore_trace]
    cli_queue: Arc<Mutex<Vec<CliCall>>>,
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
        class.method(JsString::from("globalKeyboardEvents"), 0, NativeFunction::from_fn_ptr(|this, _args, _context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let request = obj.downcast_ref::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not a RefreshRequest").into()))?;
            let mut capture = request.capture_keyboard.lock().unwrap();
            *capture = true;
            Ok(JsValue::undefined())
        }));
        class.method(JsString::from("localKeyboardEvents"), 0, NativeFunction::from_fn_ptr(|this, _args, _context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let request = obj.downcast_ref::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not a RefreshRequest").into()))?;
            let mut capture = request.capture_keyboard.lock().unwrap();
            *capture = true;
            Ok(JsValue::undefined())
        }));
        class.method(JsString::from("localKeyEvents"), 0, NativeFunction::from_fn_ptr(|this, _args, _context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let request = obj.downcast_ref::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not a RefreshRequest").into()))?;
            let mut capture = request.capture_keyboard.lock().unwrap();
            *capture = true;
            Ok(JsValue::undefined())
        }));
        class.method(JsString::from("localClickEvents"), 0, NativeFunction::from_fn_ptr(|this, _args, _context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let request = obj.downcast_ref::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not a RefreshRequest").into()))?;
            let mut capture = request.capture_clicks.lock().unwrap();
            *capture = true;
            Ok(JsValue::undefined())
        }));
        class.method(JsString::from("jsonHttpGet"), 2, NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let request = obj.downcast_ref::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not a RefreshRequest").into()))?;
            let url = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();
            let mut headers = HashMap::new();
            if let Some(h_obj) = args.get_or_undefined(1).as_object() {
                for key in h_obj.own_property_keys(context)? {
                    let k = key.to_string();
                    let v = h_obj.get(key, context)?.to_string(context)?.to_std_string().unwrap();
                    headers.insert(k, v);
                }
            }
            request.http_queue.lock().unwrap().push(HttpCall { url, headers, method: HttpMethod::Get });
            Ok(JsValue::undefined())
        }));
        class.method(JsString::from("jsonHttpPost"), 3, NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let request = obj.downcast_ref::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not a RefreshRequest").into()))?;
            let url = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();
            let mut headers = HashMap::new();
            if let Some(h_obj) = args.get_or_undefined(1).as_object() {
                for key in h_obj.own_property_keys(context)? {
                    let k = key.to_string();
                    let v = h_obj.get(key, context)?.to_string(context)?.to_std_string().unwrap();
                    headers.insert(k, v);
                }
            }
            let body = args.get_or_undefined(2).to_string(context)?.to_std_string().unwrap();
            request.http_queue.lock().unwrap().push(HttpCall { url, headers, method: HttpMethod::Post(body) });
            Ok(JsValue::undefined())
        }));
        class.method(JsString::from("CliInvoke"), 1, NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let request = obj.downcast_ref::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not a RefreshRequest").into()))?;
            let command = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();
            request.cli_queue.lock().unwrap().push(CliCall { command });
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
                let key_str = key.to_string();
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
            Ok(JsValue::undefined())
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
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let api = obj.downcast_ref::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not a WidgetAPI").into()))?;
            let id = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();
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
    #[unsafe_ignore_trace]
    states_file: PathBuf,
}

impl Class for WidgetState {
    const NAME: &'static str = "WidgetState";
    fn data_constructor(_this: &JsValue, _args: &[JsValue], _context: &mut JsContext) -> JsResult<Self> {
        Err(JsError::from_opaque(JsString::from("Cannot construct WidgetState directly").into()))
    }
    fn init(class: &mut ClassBuilder<'_>) -> JsResult<()> {
        class.method(JsString::from("setGlobalPersistence"), 2, NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let state = obj.downcast_ref::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not a WidgetState").into()))?;
            let key = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();
            let val = args.get_or_undefined(1).to_string(context)?.to_std_string().unwrap();

            let mut global_data: HashMap<String, String> = if state.states_file.exists() {
                let f = fs::File::open(&state.states_file).unwrap();
                serde_yaml::from_reader(f).unwrap_or_default()
            } else {
                HashMap::new()
            };

            global_data.insert(key, val);
            if let Ok(f) = fs::File::create(&state.states_file) {
                serde_yaml::to_writer(f, &global_data).ok();
            }
            Ok(JsValue::undefined())
        }));
        class.method(JsString::from("getGlobalPersistence"), 1, NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let state = obj.downcast_ref::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not a WidgetState").into()))?;
            let key = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();

            let global_data: HashMap<String, String> = if state.states_file.exists() {
                let f = fs::File::open(&state.states_file).unwrap();
                serde_yaml::from_reader(f).unwrap_or_default()
            } else {
                HashMap::new()
            };

            let val = global_data.get(&key).cloned().unwrap_or_default();
            Ok(JsString::from(val).into())
        }));
        class.method(JsString::from("set"), 2, NativeFunction::from_fn_ptr(|this, args, context| {
            let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
            let state = obj.downcast_ref::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not a WidgetState").into()))?;
            let key = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();
            let val = args.get_or_undefined(1).to_string(context)?.to_std_string().unwrap();
            let mut data = state.data.lock().unwrap();
            data.insert(key, val);
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
            data.insert(key, stringified);
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

pub struct WayWidget {
    pub registry_state: RegistryState,
    pub seat_state: SeatState,
    pub output_state: OutputState,
    pub _compositor_state: CompositorState,
    pub _shm_state: Shm,
    pub _xdg_shell_state: XdgShell,

    pub window: Window,
    pub pool: SlotPool,
    pub qh: QueueHandle<Self>,
    
    pub svg_root: Element,
    pub viewbox: (f64, f64),
    pub svg_handle: Option<rsvg::SvgHandle>,
    
    pub js_context: JsContext,
    pub api_proto: JsObject,
    pub handle_proto: JsObject,
    pub state_proto: JsObject,
    pub request_proto: JsObject,
    pub shared_ops: Arc<Mutex<HashMap<String, Vec<SvgOp>>>>,
    pub shared_state: Arc<Mutex<HashMap<String, String>>>,
    pub refresh_delay: Arc<Mutex<Option<u32>>>,
    pub capture_keyboard: Arc<Mutex<bool>>,
    pub capture_clicks: Arc<Mutex<bool>>,
    pub keys_pressed: Arc<Mutex<Vec<String>>>,
    
    pub http_queue: Arc<Mutex<Vec<HttpCall>>>,
    pub http_responses: Arc<Mutex<HashMap<String, HttpResult>>>,
    pub cli_queue: Arc<Mutex<Vec<CliCall>>>,
    pub cli_responses: Arc<Mutex<HashMap<String, CliResult>>>,

    pub pointer: Option<wl_pointer::WlPointer>,
    pub keyboard: Option<wl_keyboard::WlKeyboard>,
    pub seat: Option<wl_seat::WlSeat>,
    pub pointer_pos: (f64, f64),
    pub last_click: Option<(f64, f64)>,
    pub is_hovering: bool,
    
    pub exit: bool,
    pub width: u32,
    pub height: u32,
    pub needs_redraw: bool,
    
    pub widget_name: String,
    pub positions_file: PathBuf,
    pub states_file: PathBuf,
    pub pid_file: PathBuf,
    pub current_config: WidgetConfig,
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

    fn process_queues(&mut self) {
        let h_calls = { let mut lock = self.http_queue.lock().unwrap(); std::mem::take(&mut *lock) };
        process_http_queue(h_calls, self.http_responses.clone());

        let c_calls = { let mut lock = self.cli_queue.lock().unwrap(); std::mem::take(&mut *lock) };
        process_cli_queue(c_calls, self.cli_responses.clone());
    }

    pub fn draw(&mut self) {
        // 1. Process Queues
        self.process_queues();

        // 2. Get JS updates
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as f64;
        self.shared_ops.lock().unwrap().clear();
        
        let update_name = JsString::from("update");
        let global = self.js_context.global_object();
        if global.has_property(update_name.clone(), &mut self.js_context).unwrap_or(false) {
            let update_func = global.get(update_name, &mut self.js_context).unwrap();
            if let Some(func) = update_func.as_object() {
                let api_data = WidgetAPI { ops: self.shared_ops.clone(), handle_proto: self.handle_proto.clone() };
                let js_api = JsObject::from_proto_and_data(Some(self.api_proto.clone()), api_data);

                let state_data = WidgetState { data: self.shared_state.clone(), states_file: self.states_file.clone() };
                let js_state = JsObject::from_proto_and_data(Some(self.state_proto.clone()), state_data);

                let request_data = RefreshRequest { 
                    delay_ms: self.refresh_delay.clone(),
                    capture_keyboard: self.capture_keyboard.clone(),
                    capture_clicks: self.capture_clicks.clone(),
                    http_queue: self.http_queue.clone(),
                    cli_queue: self.cli_queue.clone(),
                };
                let js_request = JsObject::from_proto_and_data(Some(self.request_proto.clone()), request_data);

                // Build Response Object
                let js_response = JsObject::default(self.js_context.intrinsics());
                
                let click_val = if let Some((x, y)) = self.last_click.take() {
                    if *self.capture_clicks.lock().unwrap() {
                        let obj = JsObject::default(self.js_context.intrinsics());
                        obj.set(JsString::from("x"), JsValue::new(x), true, &mut self.js_context).ok();
                        obj.set(JsString::from("y"), JsValue::new(y), true, &mut self.js_context).ok();
                        obj.into()
                    } else { JsValue::undefined() }
                } else { JsValue::undefined() };
                js_response.set(JsString::from("click"), click_val, true, &mut self.js_context).ok();

                let mut keys_vec = self.keys_pressed.lock().unwrap();
                let js_keyboard = boa_engine::object::builtins::JsArray::new(&mut self.js_context);
                for key in keys_vec.drain(..) { js_keyboard.push(JsString::from(key), &mut self.js_context).ok(); }
                js_response.set(JsString::from("keyboard"), JsValue::from(js_keyboard), true, &mut self.js_context).ok();

                let js_cli = JsObject::default(self.js_context.intrinsics());
                let c_responses = self.cli_responses.lock().unwrap();
                for (cmd, res) in c_responses.iter() {
                    let res_obj = JsObject::default(self.js_context.intrinsics());
                    res_obj.set(JsString::from("output"), JsString::from(res.output.clone()), true, &mut self.js_context).ok();
                    if let Some(err) = &res.error { res_obj.set(JsString::from("error"), JsString::from(err.clone()), true, &mut self.js_context).ok(); }
                    js_cli.set(JsString::from(cmd.clone()), JsValue::from(res_obj), true, &mut self.js_context).ok();
                }
                js_response.set(JsString::from("cli"), JsValue::from(js_cli), true, &mut self.js_context).ok();

                let js_http = JsObject::default(self.js_context.intrinsics());
                let h_responses = self.http_responses.lock().unwrap();
                for (url, res) in h_responses.iter() {
                    let res_obj = JsObject::default(self.js_context.intrinsics());
                    res_obj.set(JsString::from("status"), JsValue::new(res.status), true, &mut self.js_context).ok();
                    res_obj.set(JsString::from("body"), JsString::from(res.body.clone()), true, &mut self.js_context).ok();
                    if let Some(err) = &res.error { res_obj.set(JsString::from("error"), JsString::from(err.clone()), true, &mut self.js_context).ok(); }
                    js_http.set(JsString::from(url.clone()), JsValue::from(res_obj), true, &mut self.js_context).ok();
                }
                js_response.set(JsString::from("http"), JsValue::from(js_http), true, &mut self.js_context).ok();
                
                func.call(&JsValue::undefined(), &[js_api.into(), JsValue::new(timestamp), js_response.into(), js_state.into(), js_request.into()], &mut self.js_context)
                    .map_err(|e| println!("JS Error in update(): {}", e)).ok();
            }
        }
        
        let ops = self.shared_ops.lock().unwrap().clone();
        let has_ops = !ops.is_empty();

        if has_ops || self.svg_handle.is_none() {
            if has_ops { apply_ops_to_svg(&mut self.svg_root, ops); }
            let mut out = Vec::new();
            self.svg_root.write(&mut out).ok();
            let bytes = glib::Bytes::from(&out);
            let stream = MemoryInputStream::from_bytes(&bytes);
            self.svg_handle = Some(Loader::new().read_stream(&stream, None as Option<&gio::File>, None as Option<&gio::Cancellable>).expect("load svg data"));
            self.needs_redraw = true;
        }

        if !self.needs_redraw || self.svg_handle.is_none() { return; }

        let (buffer, canvas) = self.pool.create_buffer(self.width as i32, self.height as i32, self.width as i32 * 4, wl_shm::Format::Argb8888).expect("create buffer");
        unsafe {
            let canvas_static: &'static mut [u8] = std::mem::transmute(canvas);
            let surface = ImageSurface::create_for_data(canvas_static, Format::ARgb32, self.width as i32, self.height as i32, self.width as i32 * 4).expect("cairo surface");
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
                let w = self.width as f64; let h = self.height as f64;
                cr.move_to(w, h - 20.0); cr.line_to(w, h); cr.line_to(w - 20.0, h); cr.close_path();
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

impl Drop for WayWidget { fn drop(&mut self) { fs::remove_file(&self.pid_file).ok(); } }

impl CompositorHandler for WayWidget {
    fn scale_factor_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: i32) {}
    fn transform_changed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: wl_output::Transform) {}
    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) { if self.needs_redraw { self.draw(); } }
    fn surface_enter(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
    fn surface_leave(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: &wl_output::WlOutput) {}
}

impl OutputHandler for WayWidget {
    fn output_state(&mut self) -> &mut OutputState { &mut self.output_state }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl ShmHandler for WayWidget { fn shm_state(&mut self) -> &mut Shm { &mut self._shm_state } }

impl WindowHandler for WayWidget {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) { self.exit = true; }
    fn configure(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window, configure: WindowConfigure, _: u32) {
        let (w, h) = configure.new_size;
        let new_w = w.map(|v| v.get()).unwrap_or(self.width);
        let new_h = h.map(|v| v.get()).unwrap_or(self.height);
        if new_w != self.width || new_h != self.height {
            self.width = new_w; self.height = new_h; self.save_positions();
        }
        self.needs_redraw = true; self.draw();
    }
}

impl SeatHandler for WayWidget {
    fn seat_state(&mut self) -> &mut SeatState { &mut self.seat_state }
    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, seat: wl_seat::WlSeat) { self.seat = Some(seat); }
    fn new_capability(&mut self, _: &Connection, qh: &QueueHandle<Self>, seat: wl_seat::WlSeat, capability: Capability) {
        if self.seat.is_none() { self.seat = Some(seat.clone()); }
        if capability == Capability::Pointer && self.pointer.is_none() { self.pointer = Some(self.seat_state.get_pointer(qh, &seat).expect("get pointer")); }
        if capability == Capability::Keyboard && self.keyboard.is_none() { self.keyboard = Some(self.seat_state.get_keyboard(qh, &seat, None).expect("get keyboard")); }
    }
    fn remove_capability(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat, capability: Capability) {
        if capability == Capability::Pointer { self.pointer = None; }
        if capability == Capability::Keyboard { self.keyboard = None; }
    }
    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) { self.seat = None; }
}

impl PointerHandler for WayWidget {
    fn pointer_frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_pointer::WlPointer, events: &[PointerEvent]) {
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
                            if px > self.width as f64 - 20.0 && py > self.height as f64 - 20.0 { self.window.xdg_toplevel().resize(seat, serial, ResizeEdge::BottomRight); }
                            else { self.window.xdg_toplevel()._move(seat, serial); }
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
delegate_keyboard!(WayWidget);
delegate_xdg_shell!(WayWidget);
delegate_xdg_window!(WayWidget);
delegate_registry!(WayWidget);

impl ProvidesRegistryState for WayWidget {
    fn registry(&mut self) -> &mut RegistryState { &mut self.registry_state }
    smithay_client_toolkit::registry_handlers![SeatState, OutputState];
}

fn get_proto<T: Class>(js_context: &mut JsContext) -> JsObject {
    js_context.global_object().get(JsString::from(T::NAME), js_context).unwrap().as_object().unwrap().get(JsString::from("prototype"), js_context).unwrap().as_object().unwrap().clone()
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let proj_dirs = ProjectDirs::from("org", "waywidget", "waywidget").ok_or_else(|| anyhow::anyhow!("Could not determine project directories"))?;
    let config_dir = proj_dirs.config_dir(); fs::create_dir_all(config_dir).ok();
    let pids_dir = config_dir.join("pids"); fs::create_dir_all(&pids_dir).ok();

    if let Some(Commands::Stop { name }) = &args.command {
        let pid_file = pids_dir.join(format!("{}.pid", name));
        if pid_file.exists() {
            let pid_str = fs::read_to_string(&pid_file)?;
            if let Ok(pid) = pid_str.trim().parse::<i32>() { println!("Stopping widget '{}' (PID: {})...", name, pid); unsafe { libc::kill(pid, libc::SIGTERM); } }
            fs::remove_file(pid_file).ok();
        } else { println!("No instance named '{}' found.", name); }
        return Ok(());
    }

    let (svg_path, script_path, width, height, widget_name, cli_pos) = match &args.command {
        Some(Commands::Run { widget, name, width, height, position }) => {
            let widget_dir = config_dir.join(widget);
            let name = name.clone().unwrap_or_else(|| widget.clone());
            (widget_dir.join("widget.svg"), Some(widget_dir.join("widget.js")), width.unwrap_or(200), height.unwrap_or(200), name, position.clone())
        }
        None => {
            let svg = args.svg.clone().ok_or_else(|| anyhow::anyhow!("SVG path required if not using 'run'"))?;
            let name = svg.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
            (svg, args.script.clone(), args.width, args.height, name, args.position.clone())
        }
        _ => unreachable!(),
    };

    let positions_file = config_dir.join("positions.yml");
    let states_file = config_dir.join("widgets_states.yml");
    let positions: Positions = if positions_file.exists() { let f = fs::File::open(&positions_file)?; serde_yaml::from_reader(f).unwrap_or_default() } else { Positions::default() };

    let mut cfg = positions.widgets.get(&widget_name).cloned().unwrap_or_default();
    
    if let Some(pos_str) = cli_pos {
        let parts: Vec<i32> = pos_str.split(',').filter_map(|s| s.trim().parse().ok()).collect();
        if parts.len() == 2 {
            cfg.x = parts[0];
            cfg.y = parts[1];
        }
    }

    let final_width = if cfg.width > 0 { cfg.width } else { width };
    let final_height = if cfg.height > 0 { cfg.height } else { height };
    println!("Starting widget '{}' at position: {:?}, size: {}x{}", widget_name, cfg, final_width, final_height);

    let pid_file = pids_dir.join(format!("{}.pid", widget_name));
    fs::write(&pid_file, std::process::id().to_string()).ok();

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
    window.set_title("WayWidget"); window.set_app_id("waywidget"); window.set_min_size(Some((100, 100))); window.set_max_size(Some((1200, 1200))); window.commit();

    let pool = SlotPool::new(1200 * 1200 * 4 * 2, &shm_state).expect("create pool");
    let svg_template = fs::read_to_string(&svg_path).expect("read svg");
    let svg_root = Element::parse(svg_template.as_bytes()).expect("parse svg");
    let viewbox_str = svg_root.attributes.get("viewBox").cloned().unwrap_or("0 0 100 100".to_string());
    let parts: Vec<f64> = viewbox_str.split_whitespace().filter_map(|s| s.parse().ok()).collect();
    let viewbox = if parts.len() == 4 { (parts[2], parts[3]) } else { (100.0, 100.0) };

    let mut js_context = JsContext::default();
    let log_fn = NativeFunction::from_fn_ptr(|_, args, context| { for arg in args { print!("{} ", arg.to_string(context).unwrap().to_std_string().unwrap()); } println!(); Ok(JsValue::undefined()) });
    js_context.register_global_builtin_callable(JsString::from("log_internal"), 0, log_fn).unwrap();
    js_context.eval(Source::from_bytes("var console = { log: log_internal };".as_bytes())).unwrap();

    js_context.register_global_class::<WidgetAPI>().unwrap(); js_context.register_global_class::<ElementHandle>().unwrap(); js_context.register_global_class::<WidgetState>().unwrap(); js_context.register_global_class::<RefreshRequest>().unwrap();
    
    let api_proto = get_proto::<WidgetAPI>(&mut js_context); let handle_proto = get_proto::<ElementHandle>(&mut js_context); let state_proto = get_proto::<WidgetState>(&mut js_context); let request_proto = get_proto::<RefreshRequest>(&mut js_context);
    
    let shared_ops = Arc::new(Mutex::new(HashMap::new())); let shared_state = Arc::new(Mutex::new(HashMap::new())); let refresh_delay = Arc::new(Mutex::new(None)); let capture_keyboard = Arc::new(Mutex::new(false)); let capture_clicks = Arc::new(Mutex::new(false)); let keys_pressed = Arc::new(Mutex::new(Vec::new()));
    let http_queue = Arc::new(Mutex::new(Vec::new())); let http_responses = Arc::new(Mutex::new(HashMap::new())); let cli_queue = Arc::new(Mutex::new(Vec::new())); let cli_responses = Arc::new(Mutex::new(HashMap::new()));

    if let Some(path) = &script_path { let js_source = fs::read_to_string(path).expect("read script"); js_context.eval(Source::from_bytes(js_source.as_bytes())).expect("eval script"); }

    let mut app = WayWidget {
        registry_state, seat_state, output_state, _compositor_state: compositor_state, _shm_state: shm_state, _xdg_shell_state: xdg_shell_state,
        window, pool, qh: qh.clone(), svg_root, viewbox, svg_handle: None,
        js_context, api_proto, handle_proto, state_proto, request_proto, 
        shared_ops, shared_state, refresh_delay: refresh_delay.clone(), capture_keyboard: capture_keyboard.clone(), capture_clicks: capture_clicks.clone(), keys_pressed: keys_pressed.clone(),
        http_queue, http_responses, cli_queue, cli_responses,
        pointer: None, keyboard: None, seat: None, pointer_pos: (0.0, 0.0), last_click: None, is_hovering: false,
        exit: false, width: final_width, height: final_height, needs_redraw: true,
        widget_name, positions_file: positions_file.clone(), states_file: states_file.clone(), pid_file, current_config: cfg,
    };

    let mut event_loop: EventLoop<WayWidget> = EventLoop::try_new().expect("create event loop");
    let handle = event_loop.handle();
    WaylandSource::new(conn, event_queue).insert(handle.clone()).expect("insert wayland source");

    let timer = Timer::from_duration(Duration::from_millis(10));
    handle.insert_source(timer, move |_, _, app| {
        let delay = { let mut lock = app.refresh_delay.lock().unwrap(); lock.take() };
        if let Some(ms) = delay {
            app.needs_redraw = true;
            let surface = app.window.wl_surface().clone();
            surface.frame(&app.qh, surface.clone());
            app.window.wl_surface().commit();
            TimeoutAction::ToDuration(Duration::from_millis(ms as u64))
        } else {
            // Force a draw/update cycle to catch background HTTP responses
            app.draw();
            TimeoutAction::ToDuration(Duration::from_millis(100))
        }
    }).expect("insert timer");

    while !app.exit { event_loop.dispatch(Duration::from_millis(10), &mut app).expect("dispatch"); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{svg::{apply_ops_to_svg, find_element_by_id, SvgOp}, WidgetAPI, WidgetState, ElementHandle, JsContext, get_proto};
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
        assert!(classes.contains("bar")); assert!(classes.contains("baz")); assert!(!classes.contains("foo"));
    }

    #[test]
    fn test_visibility_and_opacity() {
        let svg = r#"<svg><rect id="test" /></svg>"#;
        let mut root = Element::parse(svg.as_bytes()).unwrap();
        let mut ops = HashMap::new();
        ops.insert("test".to_string(), vec![SvgOp::SetVisible(false), SvgOp::SetOpacity(0.5)]);
        apply_ops_to_svg(&mut root, ops);
        let el = find_element_by_id(&mut root, "test").unwrap();
        assert_eq!(el.attributes.get("display").unwrap(), "none"); assert_eq!(el.attributes.get("opacity").unwrap(), "0.5");
    }

    #[test]
    fn test_append_element_and_clear() {
        let svg = r#"<svg><g id="container"></g></svg>"#;
        let mut root = Element::parse(svg.as_bytes()).unwrap();
        let mut ops = HashMap::new();
        let mut attrs = HashMap::new(); attrs.insert("id".to_string(), "child".to_string());
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
        assert!(transform.contains("rotate(45, 10, 10)")); assert!(transform.contains("translate(5, 5)")); assert!(transform.contains("scale(2)"));
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
        js_context.register_global_class::<WidgetAPI>().unwrap(); js_context.register_global_class::<ElementHandle>().unwrap(); js_context.register_global_class::<WidgetState>().unwrap(); js_context.register_global_class::<crate::RefreshRequest>().unwrap();
        let api_proto = get_proto::<WidgetAPI>(&mut js_context); let state_proto = get_proto::<WidgetState>(&mut js_context); let request_proto = get_proto::<crate::RefreshRequest>(&mut js_context);
        let shared_ops = Arc::new(Mutex::new(HashMap::new())); let shared_state = Arc::new(Mutex::new(HashMap::new())); let refresh_delay = Arc::new(Mutex::new(None)); let capture_keyboard = Arc::new(Mutex::new(false)); let capture_clicks = Arc::new(Mutex::new(false)); let http_queue = Arc::new(Mutex::new(Vec::new())); let cli_queue = Arc::new(Mutex::new(Vec::new()));
        let js_code = r#"
            function update(api, timestamp, response, state, request) {
                api.findById("rect1").setRotation(90).setOpacity(0.7);
                api.findById("group1").appendElement("circle", { id: "dynamic_circle", r: "5" });
                state.set("last_ts", timestamp.toString());
                request.refreshInMS(500);
                if (response.keyboard && response.keyboard[0] === "+Enter") { state.set("key_pressed", "true"); }
            }
        "#;
        js_context.eval(Source::from_bytes(js_code.as_bytes())).unwrap();
        let api_data = WidgetAPI { ops: shared_ops.clone(), handle_proto: get_proto::<ElementHandle>(&mut js_context) };
        let js_api = JsObject::from_proto_and_data(Some(api_proto), api_data);
        let state_data = WidgetState { data: shared_state.clone(), states_file: PathBuf::from("test_states.yml") };
        let js_state = JsObject::from_proto_and_data(Some(state_proto), state_data);
        let request_data = crate::RefreshRequest { 
            delay_ms: refresh_delay.clone(), capture_keyboard: capture_keyboard.clone(), capture_clicks: capture_clicks.clone(), http_queue: http_queue.clone(), cli_queue: cli_queue.clone(),
        };
        let js_request = JsObject::from_proto_and_data(Some(request_proto), request_data);
        let js_response = JsObject::default(js_context.intrinsics());
        let js_keyboard = boa_engine::object::builtins::JsArray::new(&mut js_context);
        js_keyboard.push(JsString::from("+Enter"), &mut js_context).ok();
        js_response.set(JsString::from("keyboard"), JsValue::from(js_keyboard), true, &mut js_context).ok();
        let update_func = js_context.global_object().get(JsString::from("update"), &mut js_context).unwrap();
        update_func.as_object().unwrap().call(&JsValue::undefined(), &[js_api.into(), JsValue::new(12345), js_response.into(), js_state.into(), js_request.into()], &mut js_context).unwrap();
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
