fn main() {
    println!("Hello, world!");
    
    // Sample code for testing Cupcake policies
    let config = load_config();
    run_application(config);
}

fn load_config() -> Config {
    // This function would load application configuration
    Config::default()
}

fn run_application(config: Config) {
    // Main application logic would go here
    println!("Running with config: {:?}", config);
}

#[derive(Debug, Default)]
struct Config {
    debug: bool,
    port: u16,
}