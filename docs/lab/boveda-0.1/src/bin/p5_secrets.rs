//! P5: strength score with `zxcvbn` and secret types that wipe themselves (`secrecy`, `zeroize`).

use secrecy::{ExposeSecret, SecretString};
use zeroize::Zeroizing;

fn main() {
    println!("== P5 zxcvbn: puntaje 0-4 ==");
    for password in [
        "",
        "123456",
        "password",
        "Tr0ub4dor&3",
        "correct horse battery staple",
        "lab-Maestra-2026",
        "ñandú-€-2026",
        "kV7#pQ2!xW9@mL4$zR8&",
    ] {
        let estimate = zxcvbn::zxcvbn(password, &[]);
        println!(
            "  {:<32} -> score {} (log10 intentos {:.1})",
            format!("{password:?}"),
            u8::from(estimate.score()),
            estimate.guesses_log10()
        );
    }

    println!("== P5 secrecy / zeroize ==");
    let master: SecretString = SecretString::from(String::from("lab-Maestra-2026"));
    println!("  Debug de SecretString: {master:?}");
    println!("  largo expuesto a proposito: {}", master.expose_secret().len());
    let buffer: Zeroizing<Vec<u8>> = Zeroizing::new(vec![0x41; 8]);
    println!("  Zeroizing<Vec<u8>> largo {} (se borra al soltarse)", buffer.len());
}
