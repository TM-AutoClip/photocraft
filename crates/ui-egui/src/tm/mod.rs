//! TM AutoClip yerel yaması — PhotoCraft (11 Eki 2026). Upstream'e dokunuş: `lib.rs` → `pub mod tm;`,
//! `menus.rs` → üç çağrı satırı (menü öğeleri, çalıştırma, etkinlik), `apps/photocraft/src/main.rs` → açılış kapısı.
//!
//! - Akış kipinde (TM AutoClip'in Görsel Üretim akışından açılış) Dosya menüsünde "Kaydet ve TM AutoClip'e dön":
//!   etkin belgeyi oturumun beklenen çıktı yoluna kaydeder (eşzamanlı), `teslim.json` yazar, programı kapatır.
//! - Düz kipte (sol menüden açılış) bu öğe GİZLİ; kullanıcı serbest.
//! - `craft-marka.json` linkleri (yalnız https) Yardım menüsünde PhotoCraft'ın kendi Discord öğesinin YANINA eklenir.

pub mod oturum;

use serde_json::{Value, json};

use crate::PhotocraftApp;
use crate::menus::MenuItem;

/// Bu programın oturumdaki adı.
pub const PROGRAM: &str = "photocraft";
/// "Kaydet ve TM AutoClip'e dön" komut kimliği.
pub const DONUS: &str = "tm.kaydetVeDon";
/// Marka linki komut öneki: `tm.link.<sıra>`.
const LINK: &str = "tm.link.";

/// Dosya menüsüne (Farklı Kaydet'in altına) dönüş öğesini, Yardım menüsüne marka linklerini ekler.
pub fn menu_ogeleri(app: &PhotocraftApp, items: &mut Vec<MenuItem>) {
    let Some(o) = oturum::simdiki() else { return };
    if o.akis_mi()
        && let Some(at) = items.iter().position(|i| i.id == "file.saveAs")
    {
        let path = items[at].path.clone();
        let oge = MenuItem {
            id: DONUS.into(),
            label: o.metin("donus", "Kaydet ve TM AutoClip'e dön").into(),
            path,
            shortcut: None,
            enabled: app.session.active().is_some(),
            checked: None,
            color: None,
        };
        items.insert(at + 1, oge);
    }
    if !o.linkler.is_empty()
        && let Some(at) = items.iter().position(|i| i.id == "help.discord")
    {
        let path = items[at].path.clone();
        for (k, l) in o.linkler.iter().enumerate() {
            let oge = MenuItem { id: format!("{LINK}{k}"), label: l.etiket.clone(), path: path.clone(), shortcut: None, enabled: true, checked: None, color: None };
            items.insert(at + 1 + k, oge);
        }
    }
}

/// Bizim komutların etkinliği (`None` = bizim değil).
pub fn etkin_mi(app: &PhotocraftApp, id: &str) -> Option<bool> {
    if id == DONUS {
        return Some(oturum::akis().is_some() && app.session.active().is_some());
    }
    id.strip_prefix(LINK).map(|_| true)
}

/// Bizim komutları çalıştırır (`None` = bizim değil).
pub fn calistir(app: &mut PhotocraftApp, ctx: &egui::Context, id: &str) -> Option<Result<Value, String>> {
    if let Some(sira) = id.strip_prefix(LINK) {
        let adres = oturum::simdiki().and_then(|o| sira.parse::<usize>().ok().and_then(|k| o.linkler.get(k))).map(|l| l.adres.clone());
        return Some(match adres {
            Some(a) => Ok(crate::links::open(app, ctx, &a)),
            None => Err("bilinmeyen link".into()),
        });
    }
    if id != DONUS {
        return None;
    }
    Some(kaydet_ve_don(app, ctx))
}

fn kaydet_ve_don(app: &mut PhotocraftApp, ctx: &egui::Context) -> Result<Value, String> {
    let o = oturum::akis().ok_or("TM AutoClip akış oturumu yok")?;
    let (_, yol) = o.cikti.clone().ok_or("oturumda çıktı yolu yok")?;
    let yol_metni = yol.to_string_lossy().into_owned();
    // Eşzamanlı kayıt: arka plan işi açılırsa teslim dosyası dosya bitmeden yazılırdı.
    let eski = app.background_jobs;
    app.background_jobs = false;
    let sonuc = app.save_as(Some(yol_metni.clone()));
    app.background_jobs = eski;
    let sonuc = sonuc.map_err(|e| {
        log::error!("TM AutoClip: kayıt başarısız: {e}");
        e
    })?;
    if sonuc.get("pending").and_then(Value::as_bool) == Some(true) || !yol.is_file() {
        return Err(format!("kayıt tamamlanmadı: {yol_metni}"));
    }
    let teslim = oturum::teslim_yaz(o, &yol).map_err(|e| {
        log::error!("TM AutoClip: {e}");
        e
    })?;
    app.ui.status = format!("TM AutoClip'e gönderildi: {yol_metni}");
    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    Ok(json!({"path": yol_metni, "teslim": teslim.to_string_lossy()}))
}
