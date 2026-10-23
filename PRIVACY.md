# Privacy Policy / Kebijakan Privasi — MangNotch

**Effective Date / Tanggal Berlaku:** October 23, 2026  
**Developer / Pengembang:** vestiapani  
**Application / Aplikasi:** MangNotch (Windows Desktop Application)

---

## 🇬🇧 English Version

### 1. Overview
**MangNotch** is an open-source desktop utility and productivity companion for Windows. We take your privacy seriously. **MangNotch does not collect, store, transmit, or sell any personal data, telemetry, or usage analytics to any external servers owned by the developer.**

### 2. Data Stored Locally on Your Device
All user configurations and productivity data are stored **100% locally** on your device inside `%AppData%\MangNotch\config.json`. This includes:
- Display, monitor, mascot, and theme preferences.
- Pomodoro timer settings and daily Water Tracker progress.
- Tasks entered into the built-in To-Do List.
- Local file paths temporarily placed into the File Shelf.

You have full control over this data and can view, modify, or delete it at any time directly through the application or by deleting the `%AppData%\MangNotch` folder.

### 3. System Permissions & Windows APIs Used
To function as an interactive desktop notch, MangNotch interacts with standard Windows APIs strictly in real-time memory (RAM) on your device:
- **Windows Global System Media Transport Controls (GSMTC):** Used locally to display the currently playing song title, artist, playback position, and album thumbnail, and to allow play/pause/skip controls.
- **Window & Display Detection (Win32 API):** Used locally to position the notch on your monitor, detect fullscreen applications/games (for auto-hide), and detect browser tab proximity (Smart Proximity Auto-Shrink).
- **System Status (Power & Keyboard State):** Used locally to display visual pop-ups (System Peeks) when Caps Lock is toggled or when your device charger is connected/disconnected.

None of this system state information ever leaves your computer.

### 4. Third-Party Services (LRCLIB API)
When the **Live Lyrics** feature is active and media is playing, MangNotch sends an anonymous HTTPS request containing only the **Song Title** and **Artist Name** to the public **LRCLIB API** (`https://lrclib.net/api/search`) to retrieve synchronized lyrics. No personal identifiers, user accounts, or device data are attached to these requests.

### 5. Open Source Transparency & Contact
MangNotch is fully open-source. You can inspect the complete source code at any time on GitHub:
- **Repository:** [https://github.com/MANG-CODER/MangNotch](https://github.com/MANG-CODER/MangNotch)
- **Website:** [https://mangnotch.vercel.app/](https://mangnotch.vercel.app/)

If you have any questions regarding this Privacy Policy, please open an issue on our [GitHub Issues](https://github.com/MANG-CODER/MangNotch/issues) page.

---

## 🇮🇩 Versi Bahasa Indonesia

### 1. Ringkasan Umum
**MangNotch** adalah aplikasi utilitas dan pendamping produktivitas *open-source* untuk Windows. Kami sangat menghargai privasi Anda. **MangNotch sama sekali tidak mengumpulkan, menyimpan, mengirimkan, atau menjual data pribadi, telemetri, maupun analitik penggunaan ke server eksternal milik pengembang.**

### 2. Data yang Disimpan Secara Lokal
Seluruh pengaturan dan data produktivitas disimpan **100% secara lokal** di perangkat Anda pada lokasi `%AppData%\MangNotch\config.json`, yang mencakup:
- Preferensi layar, monitor, karakter maskot, dan tema warna.
- Pengaturan waktu Pomodoro dan catatan konsumsi air harian (*Water Tracker*).
- Daftar tugas (*To-Do List*) yang Anda tulis.
- Lokasi (*path*) file lokal yang ditaruh sementara pada fitur *File Shelf*.

Anda memiliki kendali penuh atas data tersebut dan dapat mengubah atau menghapusnya kapan saja melalui aplikasi atau dengan menghapus folder `%AppData%\MangNotch`.

### 3. Penggunaan API Sistem Windows
Agar dapat berfungsi secara interaktif di layar atas Windows, MangNotch membaca status sistem secara *real-time* di memori lokal perangkat Anda:
- **Windows GSMTC (Media Control):** Digunakan secara lokal untuk menampilkan judul lagu, artis, durasi, dan sampul album yang sedang diputar, serta menjalankan tombol *Play/Pause/Next/Previous*.
- **Deteksi Jendela & Layar (Win32 API):** Digunakan secara lokal untuk menempatkan posisi Notch, mendeteksi aplikasi/game *fullscreen* (agar Notch otomatis bersembunyi), serta mendeteksi tab browser yang menyentuh area Notch (*Smart Proximity Auto-Shrink*).
- **Status Daya & Caps Lock:** Digunakan secara lokal untuk menampilkan indikator baterai serta pop-up singkat (*System Peeks*) saat tombol Caps Lock ditekan atau saat *charger* dihubungkan.

Tidak ada satu pun dari informasi sistem di atas yang dikirim keluar dari perangkat Anda.

### 4. Layanan Pihak Ketiga (LRCLIB API)
Saat fitur **Live Lyrics** digunakan, MangNotch mengirimkan permintaan HTTPS anonim yang hanya berisi **Judul Lagu** dan **Nama Artis** ke API publik **LRCLIB** (`https://lrclib.net/api/search`) untuk mengambil teks lirik yang tersinkronisasi. Tidak ada identitas pribadi atau data perangkat yang dikirimkan.

### 5. Transparansi Kode Sumber & Kontak
Kode sumber MangNotch terbuka sepenuhnya dan dapat diperiksa kapan saja melalui repositori resmi kami:
- **Repositori GitHub:** [https://github.com/MANG-CODER/MangNotch](https://github.com/MANG-CODER/MangNotch)
- **Website Resmi:** [https://mangnotch.vercel.app/](https://mangnotch.vercel.app/)

Jika Anda memiliki pertanyaan terkait Kebijakan Privasi ini, silakan hubungi kami melalui halaman [GitHub Issues](https://github.com/MANG-CODER/MangNotch/issues).