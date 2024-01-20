use std::io::{stdout, Result};

use crossterm::{ExecutableCommand, terminal::{EnterAlternateScreen, enable_raw_mode, LeaveAlternateScreen, disable_raw_mode}, event::{self, KeyEventKind, KeyCode}, cursor::{Hide, Show}};
use process_memory::{ Pid, TryIntoProcessHandle, PutAddress, ProcessHandle };
use ratatui::{Terminal, backend::CrosstermBackend, widgets::{Paragraph, LineGauge}, style::{Stylize, Style, Color}, layout::Rect};
use winsafe::{ GetAsyncKeyState, co::{VK, TH32CS}, prelude::kernel_Hprocesslist, HPROCESSLIST };


const LOGO: &str = "
⢠⠒⠒⠒⡄⠀⡆⠀⠀⢰ ⠀⢀⢂⠀⠀⢰⢄⠀⠀⡆⠐⠒⢲⠒⠂⠀⡆⠀⠀⢰⠀⢰⡄⠀⠀⢠⡆
⢸⠀⡀⡄⢃⠀⡇ ⠀⣸ ⠠⠓⠒⠣ ⢸⠀⠑⢄⡇⠀⠀⢸⠀⠀⠀⡇  ⢸⠀⢸⠈⢆⡰⠁⡇
⠀⠉⠉⠉⠂⠀⠈⠉⠉⠀ ⠁⠀ ⠀⠁⠈ ⠀⠀⠁⠀⠀⠀⠀⠀⠀⠈⠉⠉⠁⠀⠈⠀⠀ ⠀⠁
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣏⣉⣉ ⡇⠈⠢⡠⠊⠀
⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠇⠀⠀⠀⠇⠠⠊⠈⠢⠀";


const KEYS: &str = "
 [Q]       [W]
[ESC]   [A][S][D]
quit    navigate
";


const MIN_FOV: f32 = 30f32;
const MAX_FOV: f32 = 150f32;
const STEP: f32 = 10f32;


struct QBProcess {
	handle: ProcessHandle,
	base_addr: usize,
}

/// Find Quantum Break Process
fn find_qb() -> Option<QBProcess> {
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

	let handle = (pid as Pid).try_into_process_handle().ok()?;

	if base_addr != 0 {
		Some(QBProcess { handle, base_addr, })
	} else {
		None
	}
}

/// Write f32 to specified address
unsafe fn write_f32(handle: &ProcessHandle, addr: usize, val: f32) -> std::io::Result<()> {
	handle.put_address(addr, &val.to_ne_bytes())
}

fn apply_fov(proc: &QBProcess,fov: f32, fov_zoom: f32) -> bool {
	let fov_addr = proc.base_addr + 0x1177A68;

	let zoom = GetAsyncKeyState(VK::RBUTTON);

	unsafe { write_f32(&proc.handle, fov_addr, if zoom { fov_zoom } else { fov }) }
		.is_ok()
}


fn draw_ui(
	term: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
	fov: f32,
	fov_zoom: f32,
	selected: u8,
	qb_found: bool,
) -> Result<()> {
	term.draw(|frame| {
		let area = frame.size();

		frame.render_widget(
			Paragraph::new("Field of view (default):")
			.green(),
			Rect::new(1, 1, area.width - 2, 1),
		);

		frame.render_widget(
			LineGauge::default()
			.white()
			.gauge_style(Style::default().fg(Color::White).bg(Color::DarkGray))
			.label(&format!("{:3}", fov as u32) as &str)
			.ratio((fov as f64 - MIN_FOV as f64) / (MAX_FOV as f64 - MIN_FOV as f64)),
			Rect::new(1, 2, area.width - 2, 1),
		);

		frame.render_widget(
			Paragraph::new("Field of view (when aiming):")
			.cyan(),
			Rect::new(1, 4, 38, 1),
		);

		frame.render_widget(
			LineGauge::default()
			.white()
			.gauge_style(Style::default().fg(Color::White).bg(Color::DarkGray))
			.label(&format!("{:3}", fov_zoom as u32) as &str)
			.ratio((fov_zoom as f64 - MIN_FOV as f64) / (MAX_FOV as f64 - MIN_FOV as f64)),
			Rect::new(1, 5, area.width - 2, 1),
		);


		frame.render_widget(
			Paragraph::new(if qb_found { "Quantum Break found" } else { "Looking for Quantum Break..." })
			.fg(if qb_found { Color::Green } else { Color::Red }),
			Rect::new(1, 8, area.width - 1, 1),
		);

		// cursor
		if selected <= 3 && selected > 0 {
			frame.render_widget(
				Paragraph::new("*").red(),
				Rect::new(0, 3 * selected as u16 - 1, 1, 1),
			);
		}

		// draw logo
		for (i, line) in LOGO.lines().skip(1).enumerate() {
			frame.render_widget(
				Paragraph::new(line)
				.white(),
				Rect::new(area.width - 38, area.y + 7 + i as u16, 38, 1),
			);
		}

		// draw keys
		for (i, line) in KEYS.lines().skip(1).enumerate() {
			frame.render_widget(
				Paragraph::new(line)
				.fg(match i { 0 | 1 => Color::Yellow, _ => Color::DarkGray }),
				Rect::new((area.width-38) / 2 - 8, area.y + 7 + i as u16, 17, 1),
			);
		}

	})?;

	Ok(())
}


fn main() -> Result<()> {
	stdout().execute(EnterAlternateScreen)?;
	stdout().execute(Hide)?;
	enable_raw_mode()?;

	let mut term = Terminal::new(
		CrosstermBackend::new(stdout())
	)?;

	let mut fov = 100f32;
	let mut fov_zoom = 50f32;
	let mut selected = 0u8;

	draw_ui(&mut term, fov, fov_zoom, selected, false)?;

	let mut proc = None;

	loop {
		if event::poll(std::time::Duration::from_millis(0))? {
			if let event::Event::Key(key) = event::read()? {
				if key.kind == KeyEventKind::Press {
					match key.code {
						KeyCode::Esc | KeyCode::Char('q') => break,
						KeyCode::Up | KeyCode::Char('w') => if selected > 0 { selected -= 1 },
						KeyCode::Down | KeyCode::Char('s') => if selected < 2 { selected += 1 },
						KeyCode::Left | KeyCode::Char('a') => match selected {
							1 => { fov = (fov - STEP).max(MIN_FOV) },
							2 => { fov_zoom = (fov_zoom - STEP).max(MIN_FOV) }
							_ => {}
						},
						KeyCode::Right | KeyCode::Char('d') => match selected {
							1 => { fov = (fov + STEP).min(MAX_FOV) },
							2 => { fov_zoom = (fov_zoom + STEP).min(MAX_FOV) }
							_ => {}
						},
						_ => {}
					}
				}
			}

			draw_ui(&mut term, fov, fov_zoom, selected, proc.is_some())?;
		}

		if proc.is_none() {
			proc = find_qb();
			draw_ui(&mut term, fov, fov_zoom, selected, proc.is_some())?;
		}

		if let Some(p) = &proc {
			if !apply_fov(p, fov, fov_zoom) {
				proc = None;
			}
		}
	}

	stdout().execute(Show)?;
	stdout().execute(LeaveAlternateScreen)?;
	disable_raw_mode()?;

	Ok(())
}
