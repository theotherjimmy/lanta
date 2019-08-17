use lanta::{Connection, Result, Event};

fn main() -> Result<()> {
    let conn = Connection::connect()?;
    println!("{:?}", conn.list_crtc()?);
    for event in conn.get_event_loop() {
      match event {
        Event::CrtcChange(cc) => println!("{:?}", cc),
        _ => ()
      }
    }
    Ok(())
}
