fn main() {
    match sqyre_persist::Database::load_default() {
        Ok(db) => {
            println!("ok macros={}", db.macros.len());
            for name in db.macro_names() {
                let m = &db.macros[&name];
                let mut n = 0usize;
                m.root.walk(&mut |_| n += 1);
                println!("  {name}: nodes={n}");
            }
        }
        Err(e) => {
            eprintln!("fail: {e}");
            std::process::exit(1);
        }
    }
}
