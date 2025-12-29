mod openrazer;

use std::env;
use std::fs::OpenOptions;
use std::io;
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
use std::time::Duration;

use fs2::FileExt;
use ksni::menu::{Disposition, StandardItem};
use ksni::{Tray, TrayMethods};
use openrazer::Manager;

struct BatteryTray {
    counter: u8,
}

impl Tray for BatteryTray {
    fn id(&self) -> String {
        env!("CARGO_PKG_NAME").into()
    }

    // At least on gnome this isn't showing on hover so just focusing on the icon itself
    fn title(&self) -> String {
        format!("Battery {}%", self.counter)
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![render_digit_icon(self.counter)]
    }

    fn menu(&self) -> Vec<ksni::menu::MenuItem<Self>> {
        vec!(ksni::MenuItem::Standard(StandardItem {
            label: format!("Battery Level {}%", self.counter),
            enabled: false,
            visible: true,
            icon_name: "".to_string(),
            icon_data: vec![],
            shortcut: vec![],
            disposition: Disposition::Normal,
            activate: Box::new(|_this| {}),
        }))
    }
}

fn main() {
    println!("Starting rusty-razer-tray...");

    if let Err(err) = daemonize_background() {
        eprintln!("Failed to run in background: {err}");
        return;
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()
        .expect("failed to start tokio runtime");
    runtime.block_on(async_main());
}

async fn async_main() {
    let _lock_file = match acquire_single_instance_lock() {
        Ok(lock) => lock,
        Err(err) => {
            eprintln!("Another instance is already running or lock failed: {err}");
            return;
        }
    };

    let handle = BatteryTray { counter: 0 }.spawn().await.unwrap();
    let manager = match Manager::new().await {
        Ok(manager) => manager,
        Err(err) => {
            eprintln!("Failed to connect to OpenRazer via D-Bus: {err}");
            std::future::pending::<()>().await;
            return;
        }
    };

    tokio::spawn(async move {
        let mut value = 0u8;
        loop {
            if let Some(percent) = read_battery_percent(&manager).await {
                value = percent;
            }
            let _ = handle.update(|tray| tray.counter = value).await;
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
    });

    std::future::pending::<()>().await;
}

#[cfg(unix)]
fn daemonize_background() -> io::Result<()> {
    if env::var_os("RUSTY_RAZER_TRAY_FOREGROUND").is_some() {
        return Ok(());
    }

    unsafe {
        let pid = libc::fork();
        if pid < 0 {
            return Err(io::Error::last_os_error());
        }
        if pid > 0 {
            std::process::exit(0);
        }

        if libc::setsid() < 0 {
            return Err(io::Error::last_os_error());
        }

        let pid = libc::fork();
        if pid < 0 {
            return Err(io::Error::last_os_error());
        }
        if pid > 0 {
            std::process::exit(0);
        }
    }

    let devnull = OpenOptions::new().read(true).write(true).open("/dev/null")?;
    let fd = devnull.as_raw_fd();
    unsafe {
        libc::dup2(fd, libc::STDIN_FILENO);
        libc::dup2(fd, libc::STDOUT_FILENO);
        libc::dup2(fd, libc::STDERR_FILENO);
    }

    Ok(())
}

#[cfg(not(unix))]
fn daemonize_background() -> io::Result<()> {
    Ok(())
}

fn acquire_single_instance_lock() -> std::io::Result<std::fs::File> {
    let runtime_dir = env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    let lock_path = format!("{}/rusty-razer-tray.lock", runtime_dir);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(lock_path)?;
    file.try_lock_exclusive()?;
    Ok(file)
}

async fn read_battery_percent(manager: &Manager) -> Option<u8> {
    let devices = manager.get_devices().await.ok()?;
    for path in devices {
        let device = match manager.get_device(path).await {
            Ok(device) => device,
            Err(_) => continue,
        };
        if !device.has_feature("battery") {
            continue;
        }
        if let Ok(percent) = device.get_battery_percent().await {
            let percent = percent.round().clamp(0.0, 100.0) as u8;
            return Some(percent);
        }
    }
    None
}

fn render_digit_icon(value: u8) -> ksni::Icon {
    let width = 16u32;
    let height = 16u32;
    let mut data = vec![0u8; (width * height * 4) as usize];

    let outline = (255u8, 220u8, 220u8, 220u8);
    let fill = if value <= 25 {
        (255u8, 220u8, 60u8, 60u8)
    } else if value <= 50 {
        (255u8, 255u8, 224u8, 0u8)
    } else {
        (255u8, 0u8, 255u8, 0u8)
    };

    let mut set_px = |x: u32, y: u32, color: (u8, u8, u8, u8)| {
        if x >= width || y >= height {
            return;
        }
        let idx = ((y * width + x) * 4) as usize;
        data[idx] = color.0;
        data[idx + 1] = color.1;
        data[idx + 2] = color.2;
        data[idx + 3] = color.3;
    };

    draw_outlined_rect(1, 4, 12, 12, outline, &mut set_px);
    draw_outlined_rect(13, 7, 14, 9, outline, &mut set_px);

    // Battery fill (inner area).
    let inner_width = 10u32;
    let filled = (value.min(100) as u32 * inner_width) / 100;
    if filled > 0 {
        for x in 2..=1 + filled {
            for y in 5..=11 {
                set_px(x, y, fill);
            }
        }
    }

    ksni::Icon {
        width: width as i32,
        height: height as i32,
        data,
    }
}

fn draw_outlined_rect(
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    colour: (u8, u8, u8, u8),
    mut set_function: impl FnMut(u32, u32, (u8, u8, u8, u8)),
) {
    let (min_x, max_x) = if x0 <= x1 { (x0, x1) } else { (x1, x0) };
    let (min_y, max_y) = if y0 <= y1 { (y0, y1) } else { (y1, y0) };

    for x in min_x..=max_x {
        set_function(x, min_y, colour);
        set_function(x, max_y, colour);
    }

    for y in min_y..=max_y {
        set_function(min_x, y, colour);
        set_function(max_x, y, colour);
    }
}
