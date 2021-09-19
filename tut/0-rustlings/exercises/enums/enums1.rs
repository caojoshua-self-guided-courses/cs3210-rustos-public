// enums1.rs
// Make me compile! Execute `rustlings hint enums1` for hints!

#[derive(Debug)]
enum Message {
    Quit(String),
    Echo(i32),
    Move {foo: i32, bar: i32},
    ChangeColor(i32, String),
}

fn main() {
    println!("{:?}", Message::Quit(String::from("foobar")));
    println!("{:?}", Message::Echo(1));
    println!("{:?}", Message::Move{ foo: 2, bar: 3});
    println!("{:?}", Message::ChangeColor(123, String::from("barfoo")));
}
