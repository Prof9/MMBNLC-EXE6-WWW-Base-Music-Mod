pub mod memsearch;
pub mod mmbnlc;

use crate::mmbnlc::*;
use mlua::prelude::*;

static mut HOOKS: Vec<ilhook::x64::HookPoint> = Vec::new();

fn hook_direct(addr: usize, func: ilhook::x64::JmpToRetRoutine, user_data: usize) {
    let hooker = ilhook::x64::Hooker::new(
        addr,
        ilhook::x64::HookType::JmpToRet(func),
        ilhook::x64::CallbackOption::None,
        user_data,
        ilhook::x64::HookFlags::empty(),
    );
    let hook = unsafe { hooker.hook() };
    let hook = hook.expect(format!("Failed to hook {addr:#X}!").as_str());

    unsafe { &mut HOOKS }.push(hook);
}

fn hook_search(
    region: &[u8],
    what: &str,
    n: usize,
    func: ilhook::x64::JmpToRetRoutine,
) -> Result<(), ()> {
    let query = memsearch::Query::build(what).expect("query string should be valid");
    let matches = query
        .iter_matches_in(region.as_ptr() as usize, region.len())
        .take(n);
    for addr in matches {
        println!("Hooking @ {addr:#X}");
        hook_direct(addr, func, addr);
    }
    Ok(())
}

#[mlua::lua_module]
fn patch(lua: &Lua) -> LuaResult<LuaValue> {
    let text_section = lua
        .globals()
        .get::<_, LuaTable>("chaudloader")?
        .get::<_, LuaTable>("GAME_ENV")?
        .get::<_, LuaTable>("sections")?
        .get::<_, LuaTable>("text")?;
    let text_address = text_section.get::<_, LuaInteger>("address")? as usize;
    let text_size = text_section.get::<_, LuaInteger>("size")? as usize;

    hook_search(
        unsafe { std::slice::from_raw_parts(text_address as *const u8, text_size) },
        "8B4340 4533DB C1E802 A801 7516|4180F401 4180FC01 4C8D6310 750C C70361000000 EB04",
        2,
        on_hook,
    )
    .expect("Cannot find hook!");

    Ok(LuaValue::Nil)
}

unsafe extern "win64" fn on_hook(
    reg: *mut ilhook::x64::Registers,
    _return_addr: usize,
    from_addr: usize,
) -> usize {
    let gba = unsafe { GBAState::from_addr((*reg).rbx) };

    let area = gba.read_u8(gba.r6 + 4);

    // Absorb a bit of the original check so we always have enough room to place our hook
    if (*reg).r12 == 0 {
        // Normally r0 is just set to 0x61 here
        gba.r0 = match area {
            0x06 => 0x62, // If in WWW Base after beating the game, use music state with correct WWW Base music
            _ => 0x61,    // Otherwise, use music state with all other normal area music
        }
    }

    // Skip original instruction
    from_addr + 0x16
}
