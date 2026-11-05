mod config;
mod volume_ctrl;

use clap::Parser;
use gdk::gio::ApplicationFlags;
use gtk4::{self as gtk, Adjustment, EventControllerKey, Scale};

use gtk::prelude::*;
use gtk::{glib, Application, ApplicationWindow, CssProvider, Orientation};

use std::cell::RefCell;
use std::fs;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use crate::config::{Config, PartialConfig};
use crate::volume_ctrl::{Volume, VolumePeriph};

const APP_ID: &str = "com.danstuff.volumeslider";

static EXIT: AtomicBool = AtomicBool::new(false);

static STARTED: AtomicBool = AtomicBool::new(false);

static CONFIG: OnceLock<Config> = OnceLock::new();

fn main() -> glib::ExitCode {
    let config_dir = directories::ProjectDirs::from("com", "danstuff", "volumeslider")
        .map(|dirs| dirs.config_dir().to_path_buf());
    let mut part_config = PartialConfig::parse();
    let config = if let Some(config_path) = part_config.config.take() {
        part_config.join(Some(
            serde_json::from_slice(&fs::read(config_path).expect("Failed to read config path"))
                .expect("Failed to parse json"),
        ))
    } else if let Some(config_file) = config_dir {
        part_config.join({
            match fs::read(config_file.join("config.json")) {
                Ok(config) => serde_json::from_slice(&config).ok(),
                Err(_) => None,
            }
        })
    } else {
        part_config
    }
    .to_config();
    CONFIG.set(config).expect("First time setting");

    let app = Application::builder()
        .application_id(APP_ID)
        .flags(ApplicationFlags::FLAGS_NONE)
        .build();

    app.connect_activate(build_ui);

    app.run()
}

fn build_ui(app: &Application) {
    if STARTED.swap(true, Ordering::Relaxed) {
        // App already started. Don't create new window.
        return;
    }
    let config = CONFIG.get().expect("Set earlier.");

    // Register the stylesheet with GTK.
    let display = gdk::Display::default().expect("Could not connect to display");

    // Create the CSS provider.
    if let Some(style_path) = config.style.as_ref() {
        let provider = CssProvider::new();

        provider.load_from_path(style_path);

        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    let slider = Scale::new(
        Orientation::Horizontal,
        Some(&Adjustment::new(0.0, 0.0, config.max_value, 1.0, 1.0, 0.0)),
    );

    slider.set_draw_value(true);
    slider.set_increments(1.0, 10.0);
    slider.set_digits(0);

    slider.set_value_pos(gtk::PositionType::Left);

    let container = gtk::Box::new(
        if config.horizontal {
            Orientation::Horizontal
        } else {
            Orientation::Vertical
        },
        12,
    );
    container.add_css_class("content");

    container.append(&slider);

    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(config.width)
        .default_height(config.height)
        .child(&container)
        .decorated(false)
        .build();

    let key_controller = EventControllerKey::new();

    key_controller.connect_key_pressed(|_, key, _keycode, _modifiers| {
        if key == gdk::Key::Escape {
            EXIT.store(true, Ordering::Relaxed);
        }

        glib::Propagation::Proceed
    });

    // Listen for exit key.
    window.add_controller(key_controller);

    let window_clone = window.clone();

    // Check for exit.
    glib::timeout_add_local(Duration::from_millis(50), move || {
        if EXIT.load(Ordering::Relaxed) {
            window_clone.close();
            glib::ControlFlow::Break
        } else {
            glib::ControlFlow::Continue
        }
    });

    add_audio_checker_fn(&slider);
    add_on_volume_slider_change(&slider);

    setup_focus_fade(&window);

    window.present();
}

fn add_audio_checker_fn(slider: &Scale) {
    let slider = slider.clone();
    glib::timeout_add_local(Duration::from_millis(50), move || {
        let volume = Volume::get(VolumePeriph::DefaultSink);
        slider.set_value(volume.volume * 100.0);
        glib::ControlFlow::Continue
    });
}

fn add_on_volume_slider_change(slider: &Scale) {
    slider.connect_value_changed(|slider| {
        Volume::set(slider.value() / 100.0, VolumePeriph::DefaultSink);
    });
}

fn setup_focus_fade(window: &gtk::ApplicationWindow) {
    let timer: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));

    let timer_ref = timer.clone();

    window.connect_is_active_notify(move |window| {
        // Cancel any pending delay or animation.
        if let Some(source) = timer_ref.borrow_mut().take() {
            source.remove();
        }

        // Restore opacity if the window regains focus.
        if window.is_active() {
            window.set_opacity(1.0);
            return;
        }

        let window = window.clone();
        let timer = timer_ref.clone();

        // Wait one second before starting the fade.
        let source = glib::timeout_add_local_once(Duration::from_secs(1), move || {
            // Clear the ID of the completed delay.
            timer.borrow_mut().take();

            // The window may have regained focus during the delay.
            if window.is_active() {
                return;
            }

            // Capture opacity when the animation actually starts.
            let start_opacity = window.opacity();

            let start_time = Instant::now();
            let duration = Duration::from_millis(1200);

            let window = window.clone();
            let timer_ref = timer.clone();

            // Start the fade animation.
            let source = glib::timeout_add_local(Duration::from_millis(16), move || {
                // Extra safety check.
                if window.is_active() {
                    window.set_opacity(1.0);
                    timer_ref.borrow_mut().take();
                    return glib::ControlFlow::Break;
                }

                let progress =
                    (start_time.elapsed().as_secs_f64() / duration.as_secs_f64()).min(1.0);

                // Ease in/out.
                let eased = progress * progress * (3.0 - 2.0 * progress);

                let opacity = start_opacity * (1.0 - eased);

                window.set_opacity(opacity);

                if progress >= 1.0 {
                    timer_ref.borrow_mut().take();

                    if !window.is_active() {
                        window.close();
                    }

                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });

            *timer.borrow_mut() = Some(source);
        });

        *timer_ref.borrow_mut() = Some(source);
    });
}
