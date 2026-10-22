# 型付きProgram API

[English](typed-program.en.md)

[全体設計](typed-assembly.ja.md)の段階2です。同じ `Program` からASM、ELF64/COFF Object、出自対応表を作ります。Object生成はASM文字列やパーサーを経由しません。命令選択・ABI・runtimeは呼出し元が決めます。

## 入力と結果

| API・値 | 契約 |
| --- | --- |
| `Program::new(Target)` | Linux GNU／Windows MSVCのx86-64を明示。実行中OSから選ばない |
| `declare(name, Linkage)` | Program内のSymbolIdを返す。Local／Globalは一度だけ定義必須、Externalは定義禁止 |
| `push(EntryKind, Provenance)` | 入力順のEntryIdを返す。Section／Label／Instruction／Data／Align／Marker |
| `Operand::Symbol`／`RipRelative` | 明示したシンボルへの分岐／RIP相対参照。文字列から意味を推測しない |
| `assemble_program(&Program)` | Objectと項目・シンボル・参照の位置を返す |
| `render_asm(&Program)` | 同じ入力を検証し、targetに適したAT&T形式ASMを返す |
| `ProgramDiagnostic` | Validate／Encode／Resolve／Objectの段階、理由、関連EntryId・OriginId・SymbolId |

IDは各入力配列の番号です。別Program間や配列を編集した前後の同一性は保証しません。シンボル宣言はシンボル表に属し、EntryIdは付けません。外部宣言の位置は `None`、定義の位置はセクションとオフセットです。

シンボル名は `[A-Za-z_.][A-Za-z0-9_.]*`（単独の `.` を除く）です。重複名、不正ID、未定義の内部シンボル、外部シンボルの定義、データへの分岐を拒否します。`cerune_`／`.L` の接頭辞に特別な意味はありません。

## 配置と再配置

| 対象 | 実装 |
| --- | --- |
| セクション | Text／ReadOnlyData／Data。ラベル・命令・データ・整列・Markerの前に明示選択が必要 |
| データ | 生バイトを保持。NUL追加、改行変換、Unicode正規化なし。空Dataセクションも保持 |
| Align | 2の冪の指数。ELFは0〜16、COFFは0〜13。Textは0x90、他は0で埋め、Objectのセクション整列にも反映。Objectの最小整列は16バイト |
| 分岐 | call、jmp、16条件のjcc。固定の32bit変位で符号化し、Text内参照は直接解決。短い分岐への最適化なし |
| ELF | 外部分岐はPLT32、他セクション・外部データへのRIP参照はPC32。ASMの外部分岐にも `@PLT` を明示 |
| COFF | 未解決参照はREL32。命令末尾の即値を含めたaddendを反映 |
| int | 現行エンコーダと同じ `int $0x29` のみ |

同じProgram内のText定義への参照はローカルに解決します。Globalは公開属性であり、リンク時に他の定義へ差し替えるinterpositionを表しません。外部データのRIP参照がPIEで利用可能かはリンカ・配置にも依存し、GOT/TLS等は未対応です。

旧 `assemble(&str, Target)` の診断・出力は維持します。旧入力の外部jmpはPC32のままです。PLT32とPIEの実行検証は新しいProgram経路の保証です。

## 出自の読み方

`Artifact.entries` は入力順のEntryId・Provenance・半開区間 `[start, end)` を返します。位置は**生成Objectのセクション内オフセット**です。ファイル内位置やリンク後アドレスではありません。

- 1つのOriginIdから非連続の複数範囲、1つの命令から複数OriginIdを保持します。
- Syntheticは明示します。OriginIdとの併用も可能です。空のOrigin集合はSyntheticが必須、重複IDは拒否します。
- Label・Marker・空Data・padding 0は空区間を持ちます。Section切替は範囲なしです。
- `references` は参照元EntryId、SymbolId、4バイトの変位フィールド、addend、解決／再配置種別を持ちます。addendはフィールド先頭から数え、後続即値がある場合も含みます。
- ASM表示にはEntryId・OriginId・Syntheticのコメントを付けます。外部アセンブラは分岐長やpaddingを変え得るため、Objectの対応表を外部生成物へ流用しません。

有効なProvenanceだけを変更しても、Object全体は一致します。表示・符号化は入力を変更せず、対応表から実行を制御するAPIもありません。CeruneのNodeId・SourceId・Spanへの変換は呼出し元の責務です。

## 実行例と検証

`examples/program.rs` は通常call・末尾jmp・条件付き外部分岐で同じ文字列を3回表示するProgramを構築します。ABIの引数レジスタ・スタック確保は例の中で明示しています。

```sh
cargo run --example program -- x86_64-unknown-linux-gnu target/program-linux
clang -fPIE -pie target/program-linux/program.o -o target/program-linux/from-object
clang -fPIE -pie target/program-linux/program.s -o target/program-linux/from-asm
./target/program-linux/from-object
./target/program-linux/from-asm
```

Windowsではtargetを `x86_64-pc-windows-msvc` にし、対応するclang/linkerで `program.o` と `program.s` をそれぞれリンクします。例は `origins.txt` も保存します。このファイルはデバッグ表示であり、安定した交換形式ではありません。

`SASHIMI_TEST_CLANG=clang cargo test --all-targets` は外部リンク・実行比較も行います。PowerShellでは先に `$env:SASHIMI_TEST_CLANG = 'clang'` を設定します。変数がない場合は比較未実施を表示します。CIはWindows/Linuxとも設定し、ツール失敗をスキップしません。

単体テストでは両targetの原本とのObject比較、全分岐、再配置レコード、後続即値、出自の不変性、空区間、整列ヘッダ、異常入力を検証します。固定fixtureのハッシュも維持します。

## 後続の作業

Ceruneの命令選択・runtime補助はまだ既存ASM入力を使います。次は型付きProgram構築へ移し、既存135例・MIR/SSA・出自注釈・失敗診断・I/O順を比較します。

任意セクション、データ内ポインタ再配置、間接call/jmp、GOT/TLS、動的リンク、リンク後アドレスの対応表、安定API、CLIは未対応です。外部リンカや実行器をライブラリから暗黙に呼び出しません。
