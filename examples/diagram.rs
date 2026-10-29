fn main() {
    #[cfg(not(feature = "debug"))]
    {
        panic!("This example requires the `debug` feature to be enabled");
    }
    #[cfg(feature = "debug")]
    {
        use chumsky::Parser;
        use chumsky::debug::*;
        pub use openscad_parser::prelude::*;
        let parser = File::parser::<openscad_parser::prelude::FakeIterInput>()
            .debug()
            .to_railroad_svg();
        println!("{}", parser);
    }
}
