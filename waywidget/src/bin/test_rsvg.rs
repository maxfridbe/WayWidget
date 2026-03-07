use rsvg::{Loader, CairoRenderer};
fn main() {
    let handle = Loader::new().read_path("../examples/calculator/widget.svg").unwrap();
    let renderer = CairoRenderer::new(&handle);
    let rect = cairo::Rectangle::new(0.0, 0.0, 250.0, 350.0);
    // Let's try to get geometry
    match renderer.geometry_for_layer(Some("#btn-7"), &rect) {
        Ok((ink_rect, logical_rect)) => {
            println!("Ink: {:?}", ink_rect);
            println!("Logical: {:?}", logical_rect);
        }
        Err(e) => println!("Error: {:?}", e),
    }
}
