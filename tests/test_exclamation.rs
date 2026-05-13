#[test]
fn test_exclamation_marks() {
    let regular_exclamation = "!";
    let fullwidth_exclamation = "！";

    println!(
        "Regular '!': lengths: bytes={}, chars={}",
        regular_exclamation.len(),
        regular_exclamation.chars().count()
    );
    for b in regular_exclamation.bytes() {
        println!("Byte: {:#04X}", b);
    }

    println!(
        "Fullwidth '！': lengths: bytes={}, chars={}",
        fullwidth_exclamation.len(),
        fullwidth_exclamation.chars().count()
    );
    for b in fullwidth_exclamation.bytes() {
        println!("Byte: {:#04X}", b);
    }
    for c in fullwidth_exclamation.chars() {
        println!("Char: {:#06X}", c as u32);
    }
}
