mod application;
use application::Application;
use erupt::EntryLoader;

fn main() {
    println!("Program Starting!");

    let entry = EntryLoader::new().unwrap();

    let app = Application::new(&entry);

    app.run();
}
