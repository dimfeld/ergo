use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Args {
    password: String,
}

pub fn main(args: Args) -> crate::error::Result<()> {
    let hash = crate::auth::password::new_hash(&args.password)?;
    println!("{}", hash);
    Ok(())
}
