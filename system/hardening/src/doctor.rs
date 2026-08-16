use cognyx_hardening::Doctor;

fn main() {
    println!("cognyx doctor");
    for d in Doctor::run() {
        let mark = if d.ok { "ok" } else { "FAIL" };
        println!("[{mark}] {} — {} [{:?}]", d.component, d.detail, d.status);
    }
}
