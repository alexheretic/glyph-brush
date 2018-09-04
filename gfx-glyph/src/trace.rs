/// Returns a `String` backtrace from just after the `gfx_glyph` bits outwards
macro_rules! outer_backtrace {
    () => {{
        use backtrace;
        use std::fmt::Write;

        let mut on_lib = false;
        let mut outside_lib = false;
        let mut trace = String::new();
        backtrace::trace(|frame| {
            let ip = frame.ip();
            backtrace::resolve(ip, |symbol| {
                if let Some(name) = symbol.name() {
                    let name = format!("{}", name);
                    if !outside_lib && !on_lib {
                        if name.contains("gfx_glyph") {
                            on_lib = true;
                        }
                    } else if on_lib {
                        if !name.contains("gfx_glyph") {
                            outside_lib = true;
                        }
                    }

                    if outside_lib {
                        if !trace.is_empty() {
                            writeln!(trace).unwrap();
                        }
                        write!(trace, " - {}", name).unwrap();
                        if let (Some(file), Some(lineno)) = (symbol.filename(), symbol.lineno()) {
                            write!(trace, " at {:?}:{}", file, lineno).unwrap();
                        }
                    }
                }
            });
            true
        });

        trace
    }};
}
