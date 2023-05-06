use ilhook::x64::{Hooker, HookPoint, HookType, JmpToRetRoutine, Registers, CallbackOption, HookFlags};
use std::os::raw::{c_int, c_void};
use winapi::{shared::minwindef::{BOOL, DWORD, HINSTANCE, LPVOID, TRUE}, um::winnt::DLL_PROCESS_DETACH};

mod memsearch;

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct GBAState {
    pub r0:  u32,  pub r1:  u32,  pub r2:  u32,  pub r3:  u32,
    pub r4:  u32,  pub r5:  u32,  pub r6:  u32,  pub r7:  u32,
    pub r8:  u32,  pub r9:  u32,  pub r10: u32,  pub r11: u32,
    pub r12: u32,  pub r13: u32,  pub r14: u32,  pub r15: u32,
    pub flags1: u32,  pub flags2: u32,
    pub ram: *const u8,
    // TODO more stuff
}

impl GBAState {
    pub fn ram_u8(&self, addr: u32) -> u8 {
        unsafe { *(self.ram.offset(addr.try_into().unwrap())) }
    }

    pub fn from_addr<'a>(addr: u64) -> &'a mut Self {
        unsafe { &mut *(addr as *mut Self) }
    }
}

static mut HOOKS: Vec<HookPoint> = Vec::new();

#[no_mangle]
pub extern "system" fn DllMain(_module: HINSTANCE, call_reason: DWORD, _reserved: LPVOID) -> BOOL {
    if call_reason == DLL_PROCESS_DETACH {
        unsafe { &mut HOOKS }.clear();
    }
    TRUE
}

fn hook_direct(addr: usize, func: JmpToRetRoutine) {
    println!("Hooking {addr:#X}");
    let hooker = Hooker::new(
        addr,
        HookType::JmpToRet(func),
        CallbackOption::None,
        0,
        HookFlags::empty()
    );
    let hook = unsafe { hooker.hook() };
    let hook = hook.expect(format!("Failed to hook {addr:#X}!").as_str());

    unsafe { &mut HOOKS }.push(hook);
}

fn hook_search(what: &str, func: JmpToRetRoutine) {
    println!("Searching for: {what}");
    let matches = memsearch::search(what, 0x140000000, 0x10000000)
        .expect(format!("Failed to find: {what}").as_str());
    for addr in matches.iter() {
        hook_direct(*addr, func);
    }
}

#[no_mangle]
pub unsafe extern "C" fn luaopen_patch(_: c_void) -> c_int {
    hook_search("FC 01 4C 8D 63 10 75 0C|C7 03 61 00 00 00 EB 04", on_hook);
    0
}

extern "win64" fn on_hook(reg: *mut Registers, ori_func_ptr: usize, _user_data: usize) -> usize {
    let gba = unsafe { GBAState::from_addr((*reg).rbx) };

    let area = gba.ram_u8(gba.r6 + 4);

    gba.r0 = match area {
        0x06 => 0x62, // If in WWW Base, use music state with correct WWW Base music
        _    => 0x61, // Otherwise, use music state with all other normal area music
    };

    // Skip original instruction
    ori_func_ptr + 6
}
