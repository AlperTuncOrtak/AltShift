use std::sync::Mutex;
use std::time::{Duration, Instant};
use keymap::LayoutId;
use crate::injector;
use crate::layout::switch_layout;
use windows::Win32::Foundation::HWND;

/// Düzeltme işleminin (Correction) sistemdeki izi.
/// Geri alma (Undo) işlemi için gereken tüm bilgileri saklar.
#[derive(Clone, Debug)]
pub struct CorrectionRecord {
    /// Düzeltmenin yapıldığı an (800ms sınırını kontrol etmek için)
    pub timestamp: Instant,
    /// Kullanıcının aslında yazdığı (hatalı kabul edilen) orijinal metin
    pub original_text: String,
    /// Düzeltme yapılmadan önceki klavye düzeni
    pub original_layout: LayoutId,
    /// Ekrana enjekte edilen düzeltilmiş metnin uzunluğu (kaç kere silmemiz gerekecek)
    pub injected_length: usize,
}

static LAST_CORRECTION: Mutex<Option<CorrectionRecord>> = Mutex::new(None);

/// Motor bir düzeltme yaptığında bu fonksiyon çağrılarak kayıt altına alınır.
pub fn record_correction(
    original_text: String,
    original_layout: LayoutId,
    injected_length: usize,
) {
    let mut record = LAST_CORRECTION.lock().unwrap();
    *record = Some(CorrectionRecord {
        timestamp: Instant::now(),
        original_text,
        original_layout,
        injected_length,
    });
}

/// Backspace basıldığında çağrılır. 
/// Eğer son düzeltmenin üzerinden 800ms geçmediyse işlemi geri alır.
/// Başarılı olursa (Geri alma tetiklendiyse) `Some(original_text)` döner (İstisna listesine eklemek için).
pub fn try_undo(hwnd: HWND) -> Option<String> {
    let mut record_guard = LAST_CORRECTION.lock().unwrap();
    
    let record = record_guard.take()?; // Kaydı al ve temizle (sadece 1 kez geri alınabilir)

    // 800 ms sınırı (WUL-22 kuralı)
    if record.timestamp.elapsed() > Duration::from_millis(800) {
        return None;
    }

    // 1. Orijinal metni geri yaz (Ekrandaki düzeltilmiş kelimeyi sil ve eskiyi yaz)
    // injected_length + 1 siliyoruz çünkü kullanıcı bir de kendisi Backspace'e bastı
    // (Ancak hook'ta Backspace'i yutuyorsak sadece injected_length silmeliyiz. 
    // Şimdilik hook'un yuttuğunu varsayarak tasarlıyoruz).
    let _ = injector::replace_text(record.injected_length, &record.original_text);

    // 2. Klavye düzenini eski haline döndür
    let _ = switch_layout(hwnd, record.original_layout);

    // 3. İstisna listesine (engine.reject) eklenmesi için kelimeyi dön
    Some(record.original_text)
}

/// Herhangi bir harfe (veya Backspace dışı bir tuşa) basıldığında
/// son düzeltme kaydı iptal edilir (Artık geri alınamaz).
pub fn cancel_undo_window() {
    let mut record = LAST_CORRECTION.lock().unwrap();
    *record = None;
}
