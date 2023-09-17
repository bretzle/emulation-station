use crate::application::Application;

mod application;
mod arm;
mod core;
mod util;

fn main() {
    let mut app = Application::new();
    app.start();
}
