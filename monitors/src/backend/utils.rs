use wayland_client::{protocol::wl_registry, Connection, Dispatch, QueueHandle};

struct AppData(pub String);
impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
    fn event(
        data: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        if let wl_registry::Event::Global { interface, .. } = event {
            if let "zwlr_output_manager_v1" = &interface[..] {
                data.0 = String::from("WLR");
            }
            if let "kde_output_device_v2" = &interface[..] {
                data.0 = String::from("KWIN");
            }
        }
    }
}
pub fn get_wl_backend() -> String {
    let backend = String::from("None");
    let mut data = AppData(backend);
    let conn = Connection::connect_to_env().unwrap();
    let display = conn.display();
    let mut queue = conn.new_event_queue();
    let handle = queue.handle();
    display.get_registry(&handle, ());
    queue.blocking_dispatch(&mut data).unwrap();
    data.0
}
