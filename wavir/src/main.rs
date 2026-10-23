#![allow(dead_code)]
mod bus;
mod cli;
mod cursors;
mod display;
mod heat;
mod html;
mod layout;
mod render;
mod svg;
mod theme;
mod types;
mod vcd;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opts = match cli::parse_args(&args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("wavir: {}", e);
            return ExitCode::from(2);
        }
    };

    let vcd = match vcd::parse(&opts.input) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("wavir: {}", e);
            return ExitCode::from(2);
        }
    };

    let theme = theme::by_name(&opts.theme);
    let out_path = opts.output.clone().unwrap_or_else(|| default_output(&opts));

    let result = if opts.html {
        html::export(&vcd, &opts, theme)
    } else {
        Ok(render::render(&vcd, &opts, theme))
    };

    match result {
        Ok(content) => {
            if let Err(e) = std::fs::write(&out_path, content) {
                eprintln!("wavir: cannot write {}: {}", out_path, e);
                return ExitCode::from(2);
            }
            println!("wrote {}", out_path);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("wavir: {}", e);
            ExitCode::from(2)
        }
    }
}

fn default_output(opts: &cli::Opts) -> String {
    let stem = opts.input.rsplit_once('.').map(|(s, _)| s).unwrap_or(&opts.input);
    let ext = if opts.html { "html" } else { "svg" };
    format!("{}.{}", stem, ext)
}
