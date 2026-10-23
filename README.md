<div align="center">
  <img src="src-tauri/icons/logo.png" alt="MangNotch Logo" width="96" height="96" style="border-radius: 22px;" />
  <h1>MangNotch 「 マン・ノッチ 」</h1>
  <p><strong>Interactive Living Notch &amp; Desktop Companion for Windows</strong><br/>Dirancang dengan estetika <em>MangNime Dusk</em> menggunakan Rust, Tauri v2, dan Win32 Native API.</p>

  <p>
    <a href="https://github.com/MANG-CODER/MangNotch/releases/latest"><img src="https://img.shields.io/badge/Download-Installer%20(.exe)-FFB7C5?style=for-the-badge&logo=windows&logoColor=120F16" alt="Download" /></a>
    <a href="https://mangnotch.vercel.app/"><img src="https://img.shields.io/badge/Website-Live%20Demo-E2D1F9?style=for-the-badge&logo=vercel&logoColor=120F16" alt="Website" /></a>
    <img src="https://img.shields.io/badge/Backend-Rust-DEA584?style=for-the-badge&logo=rust&logoColor=120F16" alt="Rust" />
    <img src="https://img.shields.io/badge/Framework-Tauri%20v2-A2E4F5?style=for-the-badge&logo=tauri&logoColor=120F16" alt="Tauri v2" />
  </p>
</div>

---

## ✨ Tentang MangNotch

**MangNotch** mengubah tepi atas layar Windows 10 & 11 menjadi ruang pendamping interaktif yang hidup tanpa menghalangi alur kerjamu. Berjalan sangat ringan di *background* dan dikendalikan penuh lewat *System Tray*, MangNotch menyatukan kontrol musik, lirik *real-time*, *native file shelf*, dan asisten produktivitas dalam satu *living island* berestetika **MangNime Dusk**.

🌐 **Coba Simulator Interaktif di Browser:** [**mangnotch.vercel.app**](https://mangnotch.vercel.app/)

---

## 🚀 Fitur Utama

- 🎵 **Windows GSMTC Media & Synced Lyrics (LRCLIB):**
  Mendeteksi lagu yang sedang diputar di Spotify, Browser (YouTube/Web), atau pemutar media Windows secara otomatis. Dilengkapi ekstraksi warna sampul album dinamis untuk efek gradasi latar belakang dan lirik berjalan *real-time*.
- 🎶 **Compact Idle Lyrics Bar:**
  Saat membuka mode lirik dan menggeser kursor keluar dari Notch, tampilan otomatis menyusut menjadi 1 baris lirik ringkas dengan sampul & judul lagu di sisi kiri.
- 🧠 **Smart Proximity Auto-Shrink:**
  Memindai posisi tab browser (*Chrome, Edge, Brave, Firefox*) dan jendela aktif menggunakan `WM_NCHITTEST`. Jika deretan tab atau tombol `+` menyentuh area Notch, Notch otomatis menyusut ke garis tipis **6px** sehingga tab di bawahnya tetap bebas diklik.
- 📂 **Native OLE File Shelf (Drag-In & Drag-Out):**
  Tarik file dari Windows Explorer ke dalam Shelf untuk disimpan sementara, klik untuk membuka langsung tanpa *security warning popup*, atau tarik keluar (*drag-out*) langsung ke WhatsApp, Discord, dan aplikasi lainnya.
- 🍅 **Pomodoro Timer & Water Wave Tank:**
  Dilengkapi pengatur waktu fokus & *short break*, visualisasi tangki air bergelombang, daftar tugas (*To-Do List*), dan synthesizer suara *chime* lembut berbasis Web Audio API dengan opsi toggle On/Off terpisah.
- 🐾 **3 Karakter Maskot & 5 Tema Warna:**
  Pilih antara **MangCat (Cyber-Neko)**, **PitVisor (Telemetry HUD)**, atau **Kitsune Spirit Orbs** dengan 5 palet cahaya (*MangNime Dusk, Matcha Emerald, Racing Red, Sunset, Ice Blue*).
- 🎮 **True Fullscreen Auto-Hide & Headless Tray Startup:**
  Hilang 100% (`SW_HIDE`) saat bermain game atau menonton video *fullscreen* (`F11`), serta berjalan senyap saat *startup* tanpa memunculkan jendela pengaturan sebelum dipanggil lewat *System Tray*.

---

## 📦 Instalasi

### Unduh Installer Siap Pakai (Direkomendasikan)
1. Buka halaman [**GitHub Releases**](https://github.com/MANG-CODER/MangNotch/releases/latest).
2. Unduh file **`MangNotch_1.0.0_x64-setup.exe`** (atau paket `.msi`).
3. Jalankan installer. Setelah selesai, MangNotch akan langsung aktif di bagian atas layar dan ikon pengaturannya dapat diakses melalui **System Tray** Windows.

---

## 🛠️ Build dari Source Code

Pastikan perangkatmu sudah terinstal **Node.js** dan **Rust (`rustup`)** beserta *C++ Build Tools for Visual Studio*.

```
# 1. Clone repositori
git clone [https://github.com/MANG-CODER/MangNotch.git](https://github.com/MANG-CODER/MangNotch.git)
cd MangNotch

# 2. Instal dependensi
npm install

# 3. Jalankan dalam mode development
npm run tauri dev

# 4. Build menjadi file Installer (.exe & .msi)
npm run tauri build
```
🔒 Privasi & Keamanan
MangNotch berjalan 100% secara lokal tanpa mengumpulkan data pribadi, analitik, maupun telemetri. Baca selengkapnya pada dokumen Privacy Policy / Kebijakan Privasi (PRIVACY.md).

👤 Pengembang
Dikembangkan oleh vestiapani sebagai bagian dari ekosistem kreatif MangNime.

🌐 Website: mangnotch.vercel.app

💻 GitHub: @MANG-CODER
