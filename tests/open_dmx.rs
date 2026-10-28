use schema_rs::{
    model::Project,
    open_dmx::{frame, transmit, Wire},
};
#[test]
fn old_output_config_remains_identical_and_usb_round_trips() {
    let p = Project::demo();
    let json = serde_json::to_value(&p.output).unwrap();
    assert_eq!(json.as_object().unwrap().len(), 3);
    let mut value = serde_json::to_value(p).unwrap();
    value["output"]["transport"] = "open_dmx".into();
    value["output"]["device"] = "DMX123".into();
    value["output"]["universe"] = 7.into();
    value["output"]["fps"] = 30.into();
    let usb: Project = serde_json::from_value(value).unwrap();
    usb.validate().unwrap();
    assert_eq!(usb.output.device, "DMX123");
    let mut bad = usb.clone();
    bad.output.fps = 44;
    assert!(bad.validate().is_err());
    bad = usb.clone();
    bad.output.device = "".into();
    assert!(bad.validate().is_err());
    bad = usb.clone();
    bad.output.universe = 32768;
    assert!(bad.validate().is_err());
}
#[derive(Default)]
struct Mock {
    events: Vec<&'static str>,
    short: bool,
    fail_break: bool,
    bytes: Vec<u8>,
}
impl Wire for Mock {
    fn break_on(&mut self) -> Result<(), String> {
        self.events.push("on");
        Ok(())
    }
    fn break_off(&mut self) -> Result<(), String> {
        self.events.push("off");
        if self.fail_break {
            Err("break failed".into())
        } else {
            Ok(())
        }
    }
    fn write(&mut self, data: &[u8]) -> Result<usize, String> {
        self.events.push("write");
        self.bytes = data.to_vec();
        Ok(if self.short { 12 } else { data.len() })
    }
}
#[test]
fn full_dmx_frame_has_start_code_padding_and_ordered_break() {
    let bytes = frame(&[255, 99]).unwrap();
    assert_eq!(bytes.len(), 513);
    assert_eq!(&bytes[..4], &[0, 255, 99, 0]);
    assert_eq!(bytes[512], 0);
    assert!(frame(&[0; 513]).is_err());
    let mut m = Mock::default();
    let mut waits = Vec::new();
    transmit(&mut m, &bytes, |d| waits.push(d)).unwrap();
    assert_eq!(m.events, ["on", "off", "write"]);
    assert!(waits[0].as_micros() >= 92);
    assert!(waits[1].as_micros() >= 12);
    assert_eq!(m.bytes, bytes);
}
#[test]
fn short_write_and_failed_break_are_errors() {
    let mut m = Mock {
        short: true,
        ..Default::default()
    };
    assert!(transmit(&mut m, &frame(&[]).unwrap(), |_| {}).is_err());
    m = Mock {
        fail_break: true,
        ..Default::default()
    };
    assert!(transmit(&mut m, &frame(&[]).unwrap(), |_| {}).is_err());
    assert_eq!(m.events, ["on", "off"]);
}
#[cfg(not(windows))]
#[test]
fn linux_reports_usb_unavailable_without_opening_anything() {
    assert!(schema_rs::open_dmx::devices().is_err());
    assert!(schema_rs::open_dmx::Sender::open("DMX123").is_err());
}
