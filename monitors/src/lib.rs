use std::sync::{Arc, RwLock, RwLockWriteGuard};
use std::time::Duration;

use dbus::arg::{self, Append, Arg, ArgType, Get};
use dbus::blocking::Connection;
use dbus::{Error, Signature};
use dbus_crossroads::IfaceBuilder;
use gtk::gdk::RGBA;
use gtk::prelude::WidgetExt;
use gtk::prelude::{BoxExt, DrawingAreaExtManual, GdkCairoContextExt};
use gtk::Orientation;
use re_set_lib::utils::plugin::{
    PluginCapabilities, PluginImplementation, PluginTestFunc, SidebarInfo,
};
use re_set_lib::utils::plugin_setup::CrossWrapper;

pub const BASE: &str = "org.Xetibo.ReSet.Daemon";
pub const DBUS_PATH: &str = "/org/Xebito/ReSet/Plugins/Monitors";
pub const INTERFACE: &str = "org.Xetibo.ReSet.Monitors";

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn capabilities() -> PluginCapabilities {
    PluginCapabilities::new(vec!["Monitors"], PluginImplementation::Both)
}

#[no_mangle]
pub extern "C" fn frontend_startup() {
    println!("frontend startup called");
}

#[no_mangle]
pub extern "C" fn frontend_shutdown() {
    println!("frontend shutdown called");
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frontend_data() -> (SidebarInfo, Vec<gtk::Box>) {
    println!("frontend data called");
    let info = SidebarInfo {
        name: "Monitors",
        icon_name: "preferences-desktop-display-symbolic",
        parent: None,
    };
    // box for the settings
    let main_box = gtk::Box::builder()
        .orientation(Orientation::Horizontal)
        .build();

    // somehow we have to call this again?
    gtk::init().unwrap();
    let drawing_area = gtk::DrawingArea::new();
    // NOTE: ensure the size is known before!
    // Otherwise the height or width inside the set_draw_func is 0!
    // E.g. nothing is drawn
    drawing_area.set_height_request(300);
    drawing_area.set_width_request(500);
    drawing_area.set_draw_func(|_, context, _, max_height| {
        let monitor_data = get_monitor_data();
        // area.dr
        for (i, monitor) in monitor_data.iter().enumerate() {
            let height = monitor.size.1 / 10;
            let width = monitor.size.0 / 10;
            let offset_x = monitor.offset.0 / 10 + i as i32 * 5;
            let offset_y = max_height - 5 - height - (monitor.offset.1 / 10);
            context.set_source_color(&RGBA::new(0.0, 0.0, 0.0, 1.0));
            let rec = gtk::gdk::Rectangle::new(offset_x, offset_y - 5, 5, height + 10);
            context.add_rectangle(&rec);
            let rec = gtk::gdk::Rectangle::new(offset_x, offset_y - 5, width + 10, 5);
            context.add_rectangle(&rec);
            let rec = gtk::gdk::Rectangle::new(offset_x, offset_y + height, width + 10, 5);
            context.add_rectangle(&rec);
            let rec = gtk::gdk::Rectangle::new(offset_x + 5 + width, offset_y - 5, 5, height + 10);
            context.add_rectangle(&rec);
            context.fill().expect("Could not fill context");

            let rec = gtk::gdk::Rectangle::new(offset_x + 5, offset_y, width, height);
            context.set_source_color(&RGBA::new(1.0, 0.0, 0.0, 1.0));
            context.add_rectangle(&rec);
            context.fill().expect("Could not fill context");
        }
    });
    drawing_area.queue_draw();
    main_box.append(&drawing_area);

    let boxes = vec![main_box];

    (info, boxes)
}

fn get_monitor_data() -> Vec<Monitor> {
    let conn = Connection::new_session().unwrap();
    let proxy = conn.with_proxy(BASE, DBUS_PATH, Duration::from_millis(1000));
    let res: Result<(Vec<Monitor>,), Error> = proxy.method_call(INTERFACE, "GetMonitors", ());
    if res.is_err() {
        return Vec::new();
    }
    res.unwrap().0
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn frontend_tests() -> Vec<PluginTestFunc> {
    println!("frontend tests called");
    vec![]
}

//backend
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn name() -> String {
    String::from("Monitors")
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn dbus_interface(cross: Arc<RwLock<CrossWrapper>>) {
    println!("dbus interface called");
    let mut cross = cross.write().unwrap();
    let interface = setup_dbus_interface(&mut cross);
    let monitors = vec![
        Monitor {
            offset: Offset(0, 0),
            size: Size(1920, 1080),
        },
        Monitor {
            offset: Offset(1920, 0),
            size: Size(2560, 1440),
        },
    ];
    cross.insert::<MonitorData>("Monitors", &[interface], MonitorData { monitors });
}

#[no_mangle]
pub extern "C" fn backend_startup() {
    println!("startup called");
}

#[no_mangle]
pub extern "C" fn backend_shutdown() {
    println!("shutdown called");
}

#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn backend_tests() -> Vec<PluginTestFunc> {
    println!("tests called");
    Vec::new()
}

// pub fn setup_dbus_interface(cross: &mut Crossroads) -> dbus_crossroads::IfaceToken<PluginData> {
pub fn setup_dbus_interface(
    cross: &mut RwLockWriteGuard<CrossWrapper>,
) -> dbus_crossroads::IfaceToken<MonitorData> {
    cross.register::<MonitorData>(
        "org.Xetibo.ReSet.Monitors",
        |c: &mut IfaceBuilder<MonitorData>| {
            c.method(
                "GetMonitors",
                (),
                ("monitors",),
                move |_, d: &mut MonitorData, ()| {
                    println!("Dbus function test called");
                    Ok((d.monitors.clone(),))
                },
            );
            c.method(
                "SetMonitors",
                ("monitors",),
                (),
                move |_, d: &mut MonitorData, (monitors,): (Vec<Monitor>,)| {
                    d.monitors = monitors;
                    Ok(())
                },
            );
        },
    )
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct MonitorData {
    pub monitors: Vec<Monitor>,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Monitor {
    pub offset: Offset,
    pub size: Size,
}

impl Append for Monitor {
    fn append_by_ref(&self, iter: &mut arg::IterAppend) {
        iter.append_struct(|i| {
            i.append(self.offset.0);
            i.append(self.offset.1);
            i.append(self.size.0);
            i.append(self.size.1);
        });
    }
}

impl<'a> Get<'a> for Monitor {
    fn get(i: &mut arg::Iter<'a>) -> Option<Self> {
        let (offset_x, offset_y, width, height) = <(i32, i32, i32, i32)>::get(i)?;
        Some(Self {
            offset: Offset(offset_x, offset_y),
            size: Size(width, height),
        })
    }
}

impl Arg for Monitor {
    const ARG_TYPE: arg::ArgType = ArgType::Struct;
    fn signature() -> Signature<'static> {
        unsafe { Signature::from_slice_unchecked("(iiii)\0") }
    }
}

impl Monitor {
    /// These coordinates are calculated from the edge of the drawing box. Ensure the rest of the
    /// window is also taken into account when passing parameters as it will otherwise evaluate to
    /// false.
    pub fn is_coordinate_within(&self, x: i32, y: i32) -> bool {
        x >= self.offset.0
            && x <= self.offset.0 + self.size.0
            && y >= self.offset.1
            && y <= self.offset.1 + self.size.1
    }
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Offset(pub i32, pub i32);

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Size(pub i32, pub i32);
