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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    svg: PathBuf,
    #[arg(short = 'j', long)]
    script: PathBuf,
    #[arg(long, default_value_t = 200)]
    width: u32,
    #[arg(long, default_value_t = 200)]
    height: u32,
    #[arg(long = "updateS", default_value_t = 0.0)]
    update_s: f64,
}

#[derive(Debug, Clone)]
enum SvgOp {
    SetRotation { angle: f64, cx: f64, cy: f64 },
    SetText(String),
    SetAttribute { name: String, value: String },
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
        class.method(
            JsString::from("setRotation"),
            3,
            NativeFunction::from_fn_ptr(|this, args, _context| {
                let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
                let handle = obj.downcast_mut::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not an ElementHandle").into()))?;
                
                let angle = args.get_or_undefined(0).as_number().unwrap_or(0.0);
                let cx = args.get_or_undefined(1).as_number().unwrap_or(50.0);
                let cy = args.get_or_undefined(2).as_number().unwrap_or(50.0);
                handle.ops.lock().unwrap().entry(handle.id.clone()).or_default().push(SvgOp::SetRotation { angle, cx, cy });
                
                Ok(this.clone())
            }),
        );

        class.method(
            JsString::from("setText"),
            1,
            NativeFunction::from_fn_ptr(|this, args, context| {
                let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
                let handle = obj.downcast_mut::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not an ElementHandle").into()))?;
                
                let text = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();
                handle.ops.lock().unwrap().entry(handle.id.clone()).or_default().push(SvgOp::SetText(text));
                
                Ok(this.clone())
            }),
        );

        class.method(
            JsString::from("setAttribute"),
            2,
            NativeFunction::from_fn_ptr(|this, args, context| {
                let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
                let handle = obj.downcast_mut::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not an ElementHandle").into()))?;
                
                let name = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();
                let value = args.get_or_undefined(1).to_string(context)?.to_std_string().unwrap();
                handle.ops.lock().unwrap().entry(handle.id.clone()).or_default().push(SvgOp::SetAttribute { name, value });
                
                Ok(this.clone())
            }),
        );

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
        class.method(
            JsString::from("findById"),
            1,
            NativeFunction::from_fn_ptr(|this, args, context| {
                let id = args.get_or_undefined(0).to_string(context)?.to_std_string().unwrap();
                let obj = this.as_object().ok_or_else(|| JsError::from_opaque(JsString::from("Not an object").into()))?;
                let api = obj.downcast_ref::<Self>().ok_or_else(|| JsError::from_opaque(JsString::from("Not a WidgetAPI").into()))?;
                
                let handle = ElementHandle {
                    id,
                    ops: api.ops.clone(),
                };
                Ok(JsObject::from_proto_and_data(Some(api.handle_proto.clone()), handle).into())
            }),
        );
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
    
    js_context: JsContext,
    js_api: JsObject,
    shared_ops: Arc<Mutex<HashMap<String, Vec<SvgOp>>>>,
    
    pointer: Option<wl_pointer::WlPointer>,
    seat: Option<wl_seat::WlSeat>,
    pointer_pos: (f64, f64),
    is_hovering: bool,
    
    exit: bool,
    width: u32,
    height: u32,
    needs_redraw: bool,
}

fn find_element_by_id<'a>(el: &'a mut Element, id: &str) -> Option<&'a mut Element> {
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

impl WayWidget {
    fn draw(&mut self) {
        // 1. Get JS updates
        let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as f64;
        self.shared_ops.lock().unwrap().clear();
        
        let update_name = JsString::from("update");
        let update_func = self.js_context.global_object().get(update_name, &mut self.js_context).unwrap();
        if let Some(func) = update_func.as_object() {
            func.call(&JsValue::undefined(), &[self.js_api.clone().into(), JsValue::new(timestamp)], &mut self.js_context).ok();
        }
        
        // 2. Apply to tree
        let ops = self.shared_ops.lock().unwrap().clone();
        for (id, el_ops) in ops {
            if let Some(el) = find_element_by_id(&mut self.svg_root, &id) {
                for op in el_ops {
                    match op {
                        SvgOp::SetRotation { angle, cx, cy } => {
                            el.attributes.insert("transform".to_string(), format!("rotate({}, {}, {})", angle, cx, cy));
                        }
                        SvgOp::SetText(text) => {
                            el.children.clear();
                            el.children.push(xmltree::XMLNode::Text(text));
                        }
                        SvgOp::SetAttribute { name, value } => {
                            el.attributes.insert(name, value);
                        }
                    }
                }
            }
        }

        // 3. Serialize tree
        let mut out = Vec::new();
        self.svg_root.write(&mut out).ok();

        // 4. Zero-Copy Drawing: Create surface directly on Wayland SHM buffer
        let (buffer, canvas) = self
            .pool
            .create_buffer(
                self.width as i32,
                self.height as i32,
                self.width as i32 * 4,
                wl_shm::Format::Argb8888,
            )
            .expect("create buffer");

        // SAFETY: We use std::mem::transmute to bypass the 'static requirement of create_for_data.
        // This is safe because we drop the surface and context before the canvas goes out of scope
        // at the end of this frame.
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

            let bytes = glib::Bytes::from(&out);
            let stream = MemoryInputStream::from_bytes(&bytes);
            let handle = Loader::new().read_stream(&stream, None as Option<&gio::File>, None as Option<&gio::Cancellable>).expect("load svg data");
            let renderer = CairoRenderer::new(&handle);
            
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
        self.width = w.map(|v| v.get()).unwrap_or(self.width);
        self.height = h.map(|v| v.get()).unwrap_or(self.height);
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
                        if let Some(seat) = &self.seat {
                            let (px, py) = self.pointer_pos;
                            if px > self.width as f64 - 20.0 && py > self.height as f64 - 20.0 {
                                self.window.xdg_toplevel().resize(seat, serial, ResizeEdge::BottomRight);
                            } else {
                                self.window.xdg_toplevel()._move(seat, serial);
                            }
                        }
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

fn main() {
    let args = Args::parse();
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
    let svg_template = fs::read_to_string(&args.svg).expect("read svg");
    let svg_root = Element::parse(svg_template.as_bytes()).expect("parse svg");
    
    // Performance: Parse viewBox once
    let viewbox_str = svg_root.attributes.get("viewBox").cloned().unwrap_or("0 0 100 100".to_string());
    let parts: Vec<f64> = viewbox_str.split_whitespace().filter_map(|s| s.parse().ok()).collect();
    let viewbox = if parts.len() == 4 { (parts[2], parts[3]) } else { (100.0, 100.0) };

    let mut js_context = JsContext::default();
    
    // Add console.log
    let log_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        for arg in args {
            print!("{} ", arg.to_string(context).unwrap().to_std_string().unwrap());
        }
        println!();
        Ok(JsValue::undefined())
    });
    let console = JsObject::default(js_context.intrinsics());
    let log_val = JsObject::from_proto_and_data(js_context.intrinsics().constructors().function().prototype(), (log_fn,));
    console.set(JsString::from("log"), log_val, true, &mut js_context).unwrap();
    js_context.global_object().set(JsString::from("console"), console, true, &mut js_context).unwrap();

    js_context.register_global_class::<WidgetAPI>().unwrap();
    js_context.register_global_class::<ElementHandle>().unwrap();
    
    let api_proto = get_proto::<WidgetAPI>(&mut js_context);
    let handle_proto = get_proto::<ElementHandle>(&mut js_context);

    // Performance: Initialize shared API object once
    let shared_ops = Arc::new(Mutex::new(HashMap::new()));
    let api_data = WidgetAPI { ops: shared_ops.clone(), handle_proto: handle_proto.clone() };
    let js_api = JsObject::from_proto_and_data(Some(api_proto.clone()), api_data);

    let js_source = fs::read_to_string(&args.script).expect("read script");
    js_context.eval(Source::from_bytes(js_source.as_bytes())).expect("eval script");

    let mut app = WayWidget {
        registry_state, seat_state, output_state,
        _compositor_state: compositor_state, _shm_state: shm_state, _xdg_shell_state: xdg_shell_state,
        window, pool, qh: qh.clone(),
        svg_root, viewbox: (viewbox.0, viewbox.1),
        js_context, js_api, shared_ops,
        pointer: None, seat: None, pointer_pos: (0.0, 0.0), is_hovering: false,
        exit: false, width: args.width, height: args.height, needs_redraw: true,
    };

    let mut event_loop: EventLoop<WayWidget> = EventLoop::try_new().expect("create event loop");
    let handle = event_loop.handle();
    WaylandSource::new(conn, event_queue).insert(handle.clone()).expect("insert wayland source");

    if args.update_s > 0.0 {
        let update_duration = Duration::from_secs_f64(args.update_s);
        let timer = Timer::from_duration(update_duration);
        handle.insert_source(timer, move |_, _, app| {
            app.needs_redraw = true;
            let surface = app.window.wl_surface().clone();
            surface.frame(&app.qh, surface.clone());
            app.window.wl_surface().commit();
            TimeoutAction::ToDuration(update_duration)
        }).expect("insert timer");
    }

    while !app.exit {
        event_loop.dispatch(Duration::from_millis(10), &mut app).expect("dispatch");
    }
}
