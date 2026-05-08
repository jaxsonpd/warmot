use warmot::jp2_convert::{ convert_file, PngSpeed };

#[test]
fn profile() {
    println!("Running Fast");
    convert_file("./tests/data/small.jp2", "./tests/data/small_output.png", PngSpeed::Fast).unwrap();

    println!("Running Slow");
    convert_file("./tests/data/small.jp2", "./tests/data/small_output.png", PngSpeed::Small).unwrap();
}