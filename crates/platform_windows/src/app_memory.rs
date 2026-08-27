use keymap::LayoutId;
use std::collections::HashMap;
use std::sync::Mutex;

/// Uygulama bazlı (exe adı) klavye düzeni hafızası.
/// Hangi uygulamada (Örn: "telegram.exe") hangi klavye düzeninin (Örn: Rusça)
/// kullanıldığını hatırlar. WUL-23 gereksinimi.
lazy_static::lazy_static! {
    static ref APP_MEMORY: Mutex<HashMap<String, LayoutId>> = Mutex::new(HashMap::new());
}

/// Motor düzeltme yaptığında, veya kullanıcı o pencerede düzeni elle değiştirdiğinde çağrılır.
/// O uygulama için tercih edilen dili (LayoutId) hafızaya kazır.
pub fn learn_app_layout(exe_name: &str, layout: LayoutId) {
    if exe_name.is_empty() {
        return;
    }

    let mut memory = APP_MEMORY.lock().unwrap();
    memory.insert(exe_name.to_lowercase(), layout);
}

/// Odak değiştiğinde (yeni bir pencereye geçildiğinde) çağrılır.
/// Eğer uygulamanın daha önceden öğrenilmiş bir dili varsa onu döndürür.
/// Hook bu değeri alıp `switch_layout` ile dili değiştirmelidir.
pub fn get_learned_layout(exe_name: &str) -> Option<LayoutId> {
    if exe_name.is_empty() {
        return None;
    }

    let memory = APP_MEMORY.lock().unwrap();
    memory.get(&exe_name.to_lowercase()).copied()
}

/// Tüm öğrenilmiş veriyi temizler (Örn: Ayarlardan sıfırlama istendiğinde)
pub fn clear_memory() {
    let mut memory = APP_MEMORY.lock().unwrap();
    memory.clear();
}
