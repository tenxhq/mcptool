use mcptool::target::Target;

fn main() {
    let test_cases = vec![
        // IPv6 with brackets
        "[::1]:8080",
        "[2001:db8::1]:443",
        "[fe80::1%eth0]:3000",
        "tcp://[::1]:9090",
        // Port-only format
        ":8080",
        "tcp://:3000",
        // Regular IPv4
        "localhost:8080",
        "192.168.1.1:443",
        // Invalid cases that should error
        "::1",   // IPv6 without brackets and port
        "[::1]", // IPv6 with brackets but no port
    ];

    println!("IPv6 Address Parsing Demo\n");

    for input in test_cases {
        print!("Parsing '{input}': ");
        match Target::parse(input) {
            Ok(target) => {
                println!("✓ Success");
                println!("  Parsed as: {target:?}");
                println!("  Display as: {target}");
            }
            Err(e) => {
                println!("✗ Error: {e}");
            }
        }
        println!();
    }
}
