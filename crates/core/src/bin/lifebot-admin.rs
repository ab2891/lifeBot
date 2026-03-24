use std::env;

use lifebot_core::LifebotService;

fn main() {
    let base = env::current_dir().expect("current directory");
    let service = LifebotService::from_env(base);

    match env::args().nth(1).as_deref() {
        Some("seed-demo") => {
            service.init().expect("initialize service");
            service.reseed_demo().expect("reseed demo");
            println!("Lifebot demo database seeded.");
        }
        _ => {
            eprintln!("Usage: cargo run -p lifebot-core --bin lifebot-admin -- seed-demo");
            std::process::exit(1);
        }
    }
}
