use clap::Parser;
use std::fs::File;           // Zum Öffnen von Dateien
use std::io::BufReader;      // Buffered Reader für effizientes Lesen
use std::io::BufRead;        // Trait für .lines() Methode
use std::io;
use clap_derive::Parser;

#[derive(Parser)]
#[command(name = "wc")]
#[command(about = "Word Count - zählt Zeilen, Wörter und Characters", long_about = None)]
struct Args {
    /// Zähle nur Zeilen
    #[arg(short = 'l', long = "lines")]
    lines: bool,

    /// Zähle nur Wörter
    #[arg(short = 'w', long = "words")]
    words: bool,

    /// Zähle nur Bytes
    #[arg(short = 'm', long = "characters")]
    characters: bool,

    /// Dateiname
    file: Option<String>,
}

fn main()-> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut zeilen_anzahl = 0;
    let mut woerter_anzahl = 0;
    let mut characters_anzahl = 0;

    let reader: Box<dyn BufRead> = match &args.file {
        Some(dateiname) => {
            println!("Datei: {}", dateiname);
            let datei = File::open(dateiname)?;
            Box::new(BufReader::new(datei))
        }
        None => {
            println!("Lese von stdin");
            Box::new(BufReader::new(io::stdin()))
        }
    };
    let default = !args.lines && !args.words && !args.characters;

    for zeile_result in reader.lines() {
        let zeile = zeile_result?;  // Fehler nach oben propagieren

        // Wenn -l Flag gesetzt:
        if args.lines || default{
            zeilen_anzahl += 1;
        }

        // Wenn -w Flag gesetzt:
        if args.words || default {
            let woerter = zeile.split_whitespace().count();
            woerter_anzahl += woerter;
        }

        // Wenn -m Flag gesetzt:
        if args.characters || default{
            characters_anzahl += zeile.chars().count() + 2;
        }
    }

    match &args.file {
        Some(dateiname) => {
            println!("{} {} {} {}", zeilen_anzahl, woerter_anzahl, characters_anzahl, dateiname);
        }
        None => {
            println!("{} {} {}", zeilen_anzahl, woerter_anzahl, characters_anzahl);
        }
    }
    Ok(())

}