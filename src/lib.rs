use ilhook::x64::{Hooker, HookPoint, HookType, JmpToRetRoutine, Registers, CallbackOption, HookFlags};
use std::os::raw::{c_int, c_void};
use winapi::{shared::minwindef::{BOOL, DWORD, HINSTANCE, LPVOID, TRUE}, um::winnt::DLL_PROCESS_DETACH};

pub mod memsearch;

#[derive(Copy, Clone, Debug)]
#[repr(C, packed)]
pub struct GBAState {
    pub r0:  u32,  pub r1:  u32,  pub r2:  u32,  pub r3:  u32,
    pub r4:  u32,  pub r5:  u32,  pub r6:  u32,  pub r7:  u32,
    pub r8:  u32,  pub r9:  u32,  pub r10: u32,  pub r11: u32,
    pub r12: u32,  pub r13: u32,  pub r14: u32,  pub r15: u32,
    pub flags: u32,  pub flags_enabled: u32,
    pub ram: *const u8,
    pub unk50: u32, pub unk54: u32, pub unk58: u32, pub unk5c: u32,
    pub ldmia_stmia_addr: u32,
    pub stack_size: u32, pub call_depth: u32,
}

impl GBAState {
    pub fn read_u8(&self, addr: u32) -> u8 {
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

fn hook_search(what: &str, n: usize, func: JmpToRetRoutine) {
    let image_base: usize = 0x140000000 as usize;
    let tls_start: usize = unsafe { *((image_base+0x1E4) as *const u32) as usize + image_base };
    let tls_size: usize = unsafe { *((image_base+0x1E8) as *const u32) as usize };

    println!("Searching for: {what}");
    let query = memsearch::Query::build(what).expect("query string should be valid");
    println!("Query built");
    let matches = query
        .iter_matches_in(tls_start, tls_size)
        .take(n);
    for addr in matches {
        hook_direct(addr, func);
    }
}

#[no_mangle]
pub unsafe extern "C" fn luaopen_patch(_: c_void) -> c_int {
    hook_search("FC 01 4C 8D 63 10 75 0C|C7 03 61 00 00 00 EB 04", 2, on_hook);
    0
}

extern "win64" fn on_hook(reg: *mut Registers, ori_func_ptr: usize, _user_data: usize) -> usize {
    // When we get here, the game has already performed a check for incident music fix
    let gba = unsafe { GBAState::from_addr((*reg).rbx) };

    let area = gba.read_u8(gba.r6 + 4);

    gba.r0 = match area {
        0x06 => 0x62, // If in WWW Base, use music state with correct WWW Base music
        _    => 0x61, // Otherwise, use music state with all other normal area music
    };

    // Skip original instruction
    ori_func_ptr + 6
}
