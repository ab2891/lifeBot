fn main() {
    if let Some(command) = std::env::args().nth(1) {
        if command == "seed-demo" {
            let base = std::env::current_dir().expect("current directory");
            let service = lifebot_core::LifebotService::from_env(base);
            service.init().expect("initialize service");
            service.reseed_demo().expect("reseed demo");
            println!("Lifebot demo database seeded.");
            return;
        }
    }

    lifebot_desktop_lib::run();
}
