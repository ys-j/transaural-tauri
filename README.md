# スピーカーでクロストークキャンセルするアプリ

サンプルアプリです。通常のステレオ音声では音が痩せて聞こえる場合が多く、あまり効果が実感できないかもしれません。バイノーラル音声だと音場がいい感じになると思います。

## 前提

バーチャルケーブルのインストールが必要です。[VB-Audio Virtual Cable](https://vb-audio.com/Cable/)でのみ動作確認しています。

```
一般のソフトウェア --> Virtual Cable Input --> 本アプリ --> スピーカー
```

<img width="802" height="2012" alt="image" src="https://github.com/user-attachments/assets/d81e19ef-ce6e-4bac-896b-46735349985c" />

## 使用ライブラリ

- [Tauri](https://github.com/tauri-apps/tauri)
- [CPAL](https://github.com/RustAudio/cpal)
- [dasp](https://github.com/RustAudio/dasp)
- [ringbuf](https://docs.rs/ringbuf/latest/ringbuf/)

アイコンその他はそのうち差し替え予定です。
