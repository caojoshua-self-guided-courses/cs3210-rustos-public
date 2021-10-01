// FIXME: Make me compile. Diff budget: 12 line additions and 2 characters.

struct ErrorA;
struct ErrorB;

enum Error {
    A(ErrorA),
    B(ErrorB)
}

fn do_a() -> Result<u16, ErrorA> {
    Err(ErrorA)
}

fn do_b() -> Result<u32, ErrorB> {
    Err(ErrorB)
}

fn do_both() -> Result<(u16, u32), Error> {
    match (do_a(), do_b()) {
        (Ok(a), Ok(b)) => Ok((a, b)),
        (Err(error), _) => Err(Error::A(error)),
        (_, Err(error)) => Err(Error::B(error))
    }
}

fn main() { }
