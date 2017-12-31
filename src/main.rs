extern crate libc;

#[cfg(test)]
#[macro_use]
extern crate lazy_static;
#[cfg(test)]
extern crate rand;

use std::ffi::CString;

mod channel;

/// This is a opaque rust equivalent for comedi_t inside libcomedi.h
enum comedi_t {}

#[link(name = "comedi")]
extern "C" {
    fn comedi_open(interface_name: *const libc::c_char) -> *const comedi_t;
    fn comedi_dio_write(it: *const comedi_t, subd: libc::c_uint, chan: libc::c_uint, bit: libc::c_uint) -> libc::c_int;
    fn comedi_dio_read(it: *const comedi_t, subd: libc::c_uint, chan: libc::c_uint, bit: *mut libc::c_uint) -> libc::c_int;
    fn comedi_data_write(it: *const comedi_t, subd: libc::c_uint, chan: libc::c_uint, range: libc::c_uint, aref: libc::c_uint, data: libc::c_uint) -> libc::c_int;
    fn comedi_data_read(it: *const comedi_t, subd: libc::c_uint, chan: libc::c_uint, range: libc::c_uint, aref: libc::c_uint, data: *mut libc::c_uint) -> libc::c_int;
}

pub enum ElevatorDirection{
    Up,
    Down,
    Stop,
}

#[derive(Debug, PartialEq, Clone, Copy)]
enum ButtonType {
    HallUp,
    HallDown,
    Cab,
}


pub struct ElevatorInterface(*const comedi_t);

unsafe impl Send for ElevatorInterface {}

impl ElevatorInterface {
    const MOTOR_SPEED: u32 = 2800;
    const N_FLOORS: u8 = 4;
    
    fn open(interface_name: &str) -> Result<Self, ()> {
        unsafe {
            let comedi = comedi_open(CString::new(interface_name).unwrap().as_ptr());
            if comedi.is_null() {
                Err(())
            } else {
                Ok(ElevatorInterface(comedi))
            }
        }
    }

    fn set_direction(&self, dir: ElevatorDirection) {
        unsafe {
            match dir {
                ElevatorDirection::Up => {
                    comedi_dio_write(self.0, channel::MOTORDIR >> 8, channel::MOTORDIR & 0xff, 0);
                    comedi_data_write(self.0, channel::MOTOR >> 8, channel::MOTOR & 0xff, 0, 0, Self::MOTOR_SPEED);
                },
                ElevatorDirection::Down => {
                    comedi_dio_write(self.0, channel::MOTORDIR >> 8, channel::MOTORDIR & 0xff, 1);
                    comedi_data_write(self.0, channel::MOTOR >> 8, channel::MOTOR & 0xff, 0, 0, Self::MOTOR_SPEED);
                },
                ElevatorDirection::Stop => {
                    comedi_data_write(self.0, channel::MOTOR >> 8, channel::MOTOR & 0xff, 0, 0, 0);
                },
            }
        }
    }

    fn read_floorsensor(&self) -> Option<u8> {
        unsafe {
            let mut data: libc::c_uint = 0;
            comedi_dio_read(self.0, channel::SENSOR_FLOOR0 >> 8, channel::SENSOR_FLOOR0 & 0xff, &mut data);
            if data != 0 {
                return Some(0);
            }
            
            comedi_dio_read(self.0, channel::SENSOR_FLOOR1 >> 8, channel::SENSOR_FLOOR1 & 0xff, &mut data);
            if data != 0 {
                return Some(1);
            }
            
            comedi_dio_read(self.0, channel::SENSOR_FLOOR2 >> 8, channel::SENSOR_FLOOR2 & 0xff, &mut data);
            if data != 0 {
                return Some(2);
            }
            
            comedi_dio_read(self.0, channel::SENSOR_FLOOR3 >> 8, channel::SENSOR_FLOOR3 & 0xff, &mut data);
            if data != 0 {
                return Some(3);
            }
            
            None
        }
    }

    fn set_floor_button_lamp(&self, button_type: ButtonType, floor: u8, on_not_off: bool) {
        assert!(floor < ElevatorInterface::N_FLOORS);
        unsafe {
            match (button_type, floor) {
                (ButtonType::HallUp, 0) => comedi_dio_write(self.0, channel::LIGHT_UP0 >> 8, channel::LIGHT_UP0 & 0xff, on_not_off as libc::c_uint),
                (ButtonType::Cab, 0) => comedi_dio_write(self.0, channel::LIGHT_COMMAND0 >> 8, channel::LIGHT_COMMAND0 & 0xff, on_not_off as libc::c_uint),
                (ButtonType::HallUp, 1) => comedi_dio_write(self.0, channel::LIGHT_UP1 >> 8, channel::LIGHT_UP1 & 0xff, on_not_off as libc::c_uint),
                (ButtonType::HallDown, 1) => comedi_dio_write(self.0, channel::LIGHT_DOWN1 >> 8, channel::LIGHT_DOWN1 & 0xff, on_not_off as libc::c_uint),
                (ButtonType::Cab, 1) => comedi_dio_write(self.0, channel::LIGHT_COMMAND1 >> 8, channel::LIGHT_COMMAND1 & 0xff, on_not_off as libc::c_uint),
                (ButtonType::HallUp, 2) => comedi_dio_write(self.0, channel::LIGHT_UP2 >> 8, channel::LIGHT_UP2 & 0xff, on_not_off as libc::c_uint),
                (ButtonType::HallDown, 2) => comedi_dio_write(self.0, channel::LIGHT_DOWN2 >> 8, channel::LIGHT_DOWN2 & 0xff, on_not_off as libc::c_uint),
                (ButtonType::Cab, 2) => comedi_dio_write(self.0, channel::LIGHT_COMMAND2 >> 8, channel::LIGHT_COMMAND2 & 0xff, on_not_off as libc::c_uint),
                (ButtonType::HallDown, 3) => comedi_dio_write(self.0, channel::LIGHT_DOWN3 >> 8, channel::LIGHT_DOWN3 & 0xff, on_not_off as libc::c_uint),
                (ButtonType::Cab, 3) => comedi_dio_write(self.0, channel::LIGHT_COMMAND3 >> 8, channel::LIGHT_COMMAND3 & 0xff, on_not_off as libc::c_uint),
                (b, f) => panic!("You tried to set lamp in non-existing button: {:?}:{} <button:floor>", b, f), //TODO: implement display for ButtonType
            };
        }
    }

    fn read_floor_button(&self, button_type: ButtonType, floor: u8) -> bool {
        assert!(floor < ElevatorInterface::N_FLOORS);
        unsafe {
            let mut data: libc::c_uint = 0;
            match (button_type, floor) {
                (ButtonType::HallUp, 0) => comedi_dio_read(self.0, channel::BUTTON_UP0 >> 8, channel::BUTTON_UP0 & 0xff, &mut data),
                (ButtonType::Cab, 0) => comedi_dio_read(self.0, channel::BUTTON_COMMAND0 >> 8, channel::BUTTON_COMMAND0 & 0xff, &mut data),
                (ButtonType::HallUp, 1) => comedi_dio_read(self.0, channel::BUTTON_UP1 >> 8, channel::BUTTON_UP1 & 0xff, &mut data),
                (ButtonType::HallDown, 1) => comedi_dio_read(self.0, channel::BUTTON_DOWN1 >> 8, channel::BUTTON_DOWN1 & 0xff, &mut data),
                (ButtonType::Cab, 1) => comedi_dio_read(self.0, channel::BUTTON_COMMAND1 >> 8, channel::BUTTON_COMMAND1 & 0xff, &mut data),
                (ButtonType::HallUp, 2) => comedi_dio_read(self.0, channel::BUTTON_UP2 >> 8, channel::BUTTON_UP2 & 0xff, &mut data),
                (ButtonType::HallDown, 2) => comedi_dio_read(self.0, channel::BUTTON_DOWN2 >> 8, channel::BUTTON_DOWN2 & 0xff, &mut data),
                (ButtonType::Cab, 2) => comedi_dio_read(self.0, channel::BUTTON_COMMAND2 >> 8, channel::BUTTON_COMMAND2 & 0xff, &mut data),
                (ButtonType::HallDown, 3) => comedi_dio_read(self.0, channel::BUTTON_DOWN3 >> 8, channel::BUTTON_DOWN3 & 0xff, &mut data),
                (ButtonType::Cab, 3) => comedi_dio_read(self.0, channel::BUTTON_COMMAND3 >> 8, channel::BUTTON_COMMAND3 & 0xff, &mut data),
                (b, f) => panic!("You tried to set lamp in non-existing button: {:?}:{} <button:floor>", b, f), //TODO: implement display for ButtonType
            };
            data != 0
        }
    }

    fn set_stop_button_lamp(&self, on_not_off: bool) {
        unsafe {
            comedi_dio_write(self.0, channel::LIGHT_STOP >> 8, channel::LIGHT_STOP & 0xff, on_not_off as libc::c_uint);
        }
    }

    fn read_stop_button(&self) -> bool {
        unsafe{
            let mut data: libc::c_uint = 0;
            comedi_dio_read(self.0, channel::STOP >> 8, channel::STOP & 0xff, &mut data);
            data != 0
        }
    }
}

fn main() {
    let interface = ElevatorInterface::open("/dev/comedi0").unwrap();
}

#[cfg(test)]
mod tests {
    use *;

    use std::sync::Mutex;
    use std::thread;
    use std::time::Duration;

    // These tests are executed on an actual elevator. To make sure only one test is run at the same time, the elevator is protected by this mutex.
    lazy_static! {
        static ref ELEVATOR: Mutex<ElevatorInterface> = Mutex::new(ElevatorInterface::open("/dev/comedi0").unwrap());
    }
    
    
    #[test]
    fn init_elevator() {
        ELEVATOR.lock().unwrap();
    }

    #[test]
    fn test_run() {
        let elevator = ELEVATOR.lock().unwrap();
        println!("The elevator will now do a run from the bottom floor to the top floor. It will stop in the floor below the top floor");
        elevator.set_direction(ElevatorDirection::Down);
        while elevator.read_floorsensor() != Some(0) {}
        elevator.set_direction(ElevatorDirection::Up);
        while elevator.read_floorsensor() != Some(ElevatorInterface::N_FLOORS-1) {}
        elevator.set_direction(ElevatorDirection::Down);
        while elevator.read_floorsensor() != Some(ElevatorInterface::N_FLOORS-2) {}
        elevator.set_direction(ElevatorDirection::Stop);
    }
    
    #[test]
    fn test_cab_buttons() {
        let elevator = ELEVATOR.lock().unwrap();

        for i in rand::seq::sample_indices(&mut rand::thread_rng(), ElevatorInterface::N_FLOORS as usize, ElevatorInterface::N_FLOORS as usize).into_iter() {
            elevator.set_floor_button_lamp(ButtonType::Cab, i as u8, true);
            thread::sleep(Duration::new(0, 200000000));
            elevator.set_floor_button_lamp(ButtonType::Cab, i as u8, false);
            thread::sleep(Duration::new(0, 200000000));
            elevator.set_floor_button_lamp(ButtonType::Cab, i as u8, true);
            while !elevator.read_floor_button(ButtonType::Cab, i as u8) {}
            elevator.set_floor_button_lamp(ButtonType::Cab, i as u8, false);
        }
    }

    #[test]
    fn test_hall_up_buttons() {
        let elevator = ELEVATOR.lock().unwrap();

        for i in rand::seq::sample_indices(&mut rand::thread_rng(), ElevatorInterface::N_FLOORS as usize - 1, ElevatorInterface::N_FLOORS as usize - 1).into_iter() {
            elevator.set_floor_button_lamp(ButtonType::HallUp, i as u8, true);
            thread::sleep(Duration::new(0, 200000000));
            elevator.set_floor_button_lamp(ButtonType::HallUp, i as u8, false);
            thread::sleep(Duration::new(0, 200000000));
            elevator.set_floor_button_lamp(ButtonType::HallUp, i as u8, true);
            while !elevator.read_floor_button(ButtonType::HallUp, i as u8) {}
            elevator.set_floor_button_lamp(ButtonType::HallUp, i as u8, false);
        }
    }

    #[test]
    fn test_hall_down_buttons() {
        let elevator = ELEVATOR.lock().unwrap();

        for i in rand::seq::sample_indices(&mut rand::thread_rng(), ElevatorInterface::N_FLOORS as usize - 1, ElevatorInterface::N_FLOORS as usize - 1).into_iter() {
            elevator.set_floor_button_lamp(ButtonType::HallDown, i as u8 + 1, true);
            thread::sleep(Duration::new(0, 200000000));
            elevator.set_floor_button_lamp(ButtonType::HallDown, i as u8 + 1, false);
            thread::sleep(Duration::new(0, 200000000));
            elevator.set_floor_button_lamp(ButtonType::HallDown, i as u8 + 1, true);
            while !elevator.read_floor_button(ButtonType::HallDown, i as u8 + 1) {}
            elevator.set_floor_button_lamp(ButtonType::HallDown, i as u8 + 1, false);
        }
    }

    #[test]
    fn test_stop_button() {
        let elevator = ELEVATOR.lock().unwrap();
        
        elevator.set_stop_button_lamp(true);
        thread::sleep(Duration::new(0, 200000000));
        elevator.set_stop_button_lamp(false);
        thread::sleep(Duration::new(0, 200000000));
        elevator.set_stop_button_lamp(true);
        while !elevator.read_stop_button() {}
        elevator.set_stop_button_lamp(false);
    }

}
