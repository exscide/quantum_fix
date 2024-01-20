use process_memory::{ Pid, TryIntoProcessHandle, CopyAddress, PutAddress, ProcessHandle };
use winsafe::{ GetAsyncKeyState, co::{VK, TH32CS}, prelude::kernel_Hprocesslist, HPROCESSLIST };


/// Find Quantum Break Process
fn find_qb() -> Option<(u32, usize)> {
	let mut p = HPROCESSLIST::CreateToolhelp32Snapshot(TH32CS::SNAPPROCESS, None).ok()?;

	let mut pid = 0;

	for proc in p.iter_processes() {
		let proc = proc.ok()?;
		if proc.szExeFile() == "QuantumBreak.exe" {
			pid = proc.th32ProcessID;
			break;
		}
	}

	if pid == 0 {
		return None;
	}


	let mut p = HPROCESSLIST::CreateToolhelp32Snapshot(TH32CS::SNAPMODULE, Some(pid)).ok()?;

	let mut base_addr = 0;

	for m in p.iter_modules() {
		let m = m.ok()?;
		if m.szExePath().contains("QuantumBreak.exe") {
			base_addr = m.modBaseAddr as usize;
			break;
		}
	}


	if base_addr != 0 {
		Some((pid, base_addr))
	} else {
		None
	}
}

unsafe fn read_f32(handle: &ProcessHandle, addr: usize) -> std::io::Result<f32> {
	let mut buf = [0u8; 4];

	handle.copy_address(addr, &mut buf)?;

	Ok(f32::from_ne_bytes(buf))
}

unsafe fn write_f32(handle: &ProcessHandle, addr: usize, val: f32) -> std::io::Result<()> {
	handle.put_address(addr, &val.to_ne_bytes())
}


const FOV: f32 = 100f32;
const FOV_ZOOM: f32 = 50f32;


fn main() {
	println!("Attaching to Quantum Break...");

	loop {

		let (pid, base_addr) = match find_qb() {
			Some(p) => (p.0 as Pid, p.1),
			None => continue,
		};

		let handle = match pid.try_into_process_handle() {
			Ok(p) => p,
			Err(_) => continue,
		};

		println!("Attached");

		let fov_addr = base_addr + 0x1177A68;

		println!("cur: {}", match unsafe { read_f32(&handle, fov_addr) } {
			Ok(f) => f,
			Err(_) => continue
		});

		loop {
			let zoom = GetAsyncKeyState(VK::RBUTTON);
	
			match unsafe { write_f32(&handle, fov_addr, if zoom { FOV_ZOOM } else { FOV }) } {
				Err(_) => {
					println!("Attaching to Quantum Break...");
					break
				},
				_ => {}
			}

		}
	}

}
