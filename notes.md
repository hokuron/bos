# Writing an OS in Rust

## Learning Points

### [A Freestanding Rust Binary](https://os.phil-opp.com/freestanding-rust-binary/)

#### [The `start` attribute](https://os.phil-opp.com/freestanding-rust-binary/#the-start-attribute)

> The C runtime then invokes the [entry point of the Rust](https://github.com/rust-lang/rust/blob/bb4d1491466d8239a7a5fd68bd605e3276e97afb/src/libstd/rt.rs#L32-L73) runtime …

> … C runtime library called `crt0` (“C runtime zero”) …

> Implementing the `start` language item wouldn't help, since it would still require `crt0`. Instead, we need to **overwrite the `crt0` entry point** directly.

以上から次の `#![no_main]` attribute につながる。

#### [Overwriting the Entry Point](https://os.phil-opp.com/freestanding-rust-binary/#overwriting-the-entry-point)

> The reason for naming the function `_start` is that this is the default entry point name for most systems.

> The `!` return type … i.e. not allowed to ever return. This is required because the entry point is not called by any function, but invoked directly by the operating system or bootloader.

> To describe different environments, Rust uses a string called [target triple](https://clang.llvm.org/docs/CrossCompilation.html#target-triple).

> The above output is from a `x86_64` Linux system. We see that the `host` triple is `x86_64-unknown-linux-gnu`, which includes the CPU architecture (`x86_64`), the vendor (`unknown`), the operating system (`linux`), and the ABI (`gnu`).

- `linux`ではよく`unknown`を目にするけど、vendorのことだった
- Intel Mac は、`x86_64-apple-darwin`となる
  - CPU architecture: `x86_64`
  - vendor: `apple`
  - OS: `darwin` (macOS)

### [A Minimal Rust Kernel](https://os.phil-opp.com/minimal-rust-kernel/)

#### [BIOS Boot](https://os.phil-opp.com/minimal-rust-kernel/#bios-boot)

> Most bootloaders are larger than 512 bytes, so bootloaders are commonly split into a small first stage, which fits into 512 bytes, and a second stage, which is subsequently loaded by the first stage.

- Bootloaderのタスク
  1. Kernel image の配置と読み込み
  2. CPUのモードを切り替え最終的に64-bitにする
    - 16-bit(real mode) -> 32-bit(protected mode) -> 64-bit(long mode)
  3. BIOSからメモリマップ情報などを問合せKernelに渡す

- [`Multiboot`](https://wiki.osdev.org/Multiboot)
  - Bootloader標準
  - BootloaderとOS間のI/Fを定義
  - Linuxでよく目にするGRUBは、これのリファレンス実装

#### [A Minimal Kernel](https://os.phil-opp.com/minimal-rust-kernel/#a-minimal-kernel) - [Target Specification](https://os.phil-opp.com/minimal-rust-kernel/#target-specification)

> We're writing a kernel, so we'll need to handle interrupts at some point. To do that safely, we have to disable a certain stack pointer optimization called the “red zone”, because it would cause stack corruptions otherwise.

- 割り込み処理があるためred zoneがスタックの破損を引き起こすおそれがある
  - 関数実行のスタックフレームの最適化
  - リーフ関数(↓)に利用される(リーフ関数以外の利用用途もある？)
    - その関数内で他の関数を実行しない関数
    - 関数実行全体で最後に実行される関数
  - リーフ関数は、呼び出し元のスタックフレーム中のred zoneを利用することで、新たなスタックフレームの調整が不要になる
  - red zoneは128バイトなので、それを超えるリーフ関数では利用できない
  - 中断中の(割り込まれた)関数がred zoneを利用していた場合、例外処理がred zone(の一部？)を上書きする

> The mmx and sse features determine support for Single Instruction Multiple Data (SIMD) instructions, which can often speed up programs significantly. However, using the large SIMD registers in OS kernels leads to performance problems. The reason is that the kernel needs to restore all registers to their original state before continuing an interrupted program. This means that the kernel has to save the complete SIMD state to main memory on each system call or hardware interrupt. Since the SIMD state is very large (512–1600 bytes) and interrupts can occur very often, these additional save/restore operations considerably harm performance. To avoid this, we disable SIMD for our kernel (not for applications running on top!).
>
>A problem with disabling SIMD is that floating point operations on x86_64 require SIMD registers by default. To solve this problem, we add the soft-float feature, which emulates all floating point operations through software functions based on normal integers.

- 割り込み処理があるためSIMDがパフォーマンスの低下を引き起こす
  - 割り込みから処理を復帰する際にカーネルは、全てのレジスターの状態を元に戻す必要がある
  - SIMDが有効になっていると、その状態(サイズが512~1600バイトと大きい)をメインメモリに都度保持することになる
    - `mmx` (Multi Media eXtension): 64 bit registers (`mm0`~`mm7`)
    - `sse` (Streaming SIMD eXtensions): 128 bit registers (`xmm0`~`xmm15`)
    - `avx` (Advanced Vector eXtensions): 256 bit registers (`ymm0`~`ymm15`)
  - システムコールやハードの割り込みは高頻度で発生するため、保持と復元によるパフォーマス劣化は相当なものとなる
  - 

- x86_64は浮動小数点を扱うのにSIMD(`sse`)が必要となる
  - SIMDを無効にする代わりに`soft-float`を有効にする
  - `soft-float`は、整数を基にしたソフトウェア関数で浮動小数点演算をエミュレートする
    - ソフトウェア関数：浮動小数点はハードウェア(FPU; FloatingPointUnit)を用いるのと対比した表現？

> The problem is that the core library is distributed together with the Rust compiler as a precompiled library. So it is only valid for supported host triples (e.g., x86_64-unknown-linux-gnu) but not for our custom target. If we want to compile code for other targets, we need to recompile core for these targets first.

- サポートされたhost tripleじゃないと`core`クレートが付いてこない
  - `core`クレート: Rustコンパイラーにprecompiled libraryとして付いてくる
- `no_std`もクレートの一種(!)で、上記`core`をリンクしている(`std`クレートの代わりに？)


That's where the `build-std` feature of cargo comes in. It allows to recompile core and other standard library crates on demand, instead of using the precompiled versions shipped with the Rust installation. This feature is very new and still not finished, so it is marked as "unstable" and only available on nightly Rust compilers.

- `build-std` feature
  - Rustコンパイラーに付いてくるものとは別に明示的に`std`, `core`などのクレートを再コンパイルできる

> The Rust compiler assumes that a certain set of built-in functions is available for all systems. Most of these functions are provided by the compiler_builtins crate that we just recompiled. However, there are some memory-related functions in that crate that are not enabled by default because they are normally provided by the C library on the system.  
> ...  
> Fortunately, the compiler_builtins crate already contains implementations for all the needed functions, they are just disabled by default to not collide with the implementations from the C library. We can enable them by setting cargo's build-std-features flag to `["compiler-builtins-mem"]`

- `compiler_builtins`クレートにはメモリ管理関連の組み込み関数(memory-related intrinsics)が含まれる
  - これは`compiler_builtins`クレートの[`mem` feature](https://github.com/rust-lang/compiler-builtins/blob/eff506cd49b637f1ab5931625a33cef7e91fbbf6/Cargo.toml#L54-L55)として提供される
  - デフォルトでは無効になっており、代わりにCライブライりを使うようになっている

### [VGA Text Mode](https://os.phil-opp.com/vga-text-mode/)

- `println!()`などのテキスト出力に利用

#### [The VGA Text Buffer](https://os.phil-opp.com/vga-text-mode/#the-vga-text-buffer)

- 80列(文字?)を25行分描画するためのバッファー
- 概ねASCIIの範囲 ([正確には違うらしい](https://en.wikipedia.org/wiki/Code_page_437))
- 文字色や背景色の指定、文字を点滅させることも可能
  - 文字色の各色は、その指定色のより明るい色にすることも可能
  - 文字色は色指定に3ビット、明るさ指定(フラグ)に1ビットの計4ビットを利用する
  - 背景色は色指定の3ビットのみで、残り1ビットには文字の点滅フラグに利用される
- このバッファーへのアクセスには、RAMを介さずに直接VGAハードウェアのアドレス空間(`0xb8000`)にアクセスする
  - これをMemory-mapped I/Oという
  - このおかげで対象のアドレス(`0xb8000`)に対する通常のメモリ操作を行うことで読み書きが可能になる

#### [Volatile](https://os.phil-opp.com/vga-text-mode/#volatile)

> we only write to the Buffer and never read from it again. The compiler doesn't know that we really access VGA buffer memory (instead of normal RAM) and knows nothing about the side effect that some characters appear on the screen. So it might decide that these writes are unnecessary and can be omitted.

- Memory-mapped I/Oを利用し、RAMではなくVGAハードウェアのアドレスに直接書き込んでいるため、`Buffer`への参照(Read)がコード中に現れない
- コンパイラーの最適化によって書き込み部分が削除(omit)されてしまう
- これを避けるために`core`クレートには[`read_volatile`](https://doc.rust-lang.org/nightly/core/ptr/fn.read_volatile.html)と[`write_volatile`](https://doc.rust-lang.org/nightly/core/ptr/fn.write_volatile.html)が用意されている
- それを内部で利用する`Volatile`ラッパータイプを提供する[`volatile`クレート](https://docs.rs/volatile)が便利

#### [A Global Interface](https://os.phil-opp.com/vga-text-mode/#a-global-interface)

- `const`と`static`の初期化はcompile-timeに行われる
  - `const`はインライン展開される
  - `static`は常に固定のアドレスを指す
    - 可変にするにはこちらを選ぶ (`static mut`)
    - `static mut`の場合、スレッド間で競合するおそれがあるため、変更操作時には`unsafe`が伴う
    - 不変な`static`はスレッドセーフであることを証明?するために`Sync` traitを実装する必要がある
  - 定数評価では生のポインターを(Rustの)参照に変換できない
- `lazy_static`クレート
  - `static`の初期化を初回アクセス時まで遅延、つまりruntime評価にする
  - `static ref`となる(runtime評価だから?)ため可変性がなくなる
    - 各種`Cell`のinterior mutability(内部可変)は`Sync` traitを実装していないため利用できない
    - 代わりに`a`を使う
      - カーネルなので標準ライブラリで提供されているのは使えない
      - OSの機能に依らないより基礎的な["Spinlock"](https://ja.wikipedia.org/wiki/スピンロック)と呼ばれるmutex(を実装したクレート`spin`)が利用できる
        - スレッドをブロックするのではなく、スレッドは単純に短時間のうちにリソースのロックを何度も試みようとする(その分ロックが解放されるまでCPU timeを消費する)

#### [A println Macro](https://os.phil-opp.com/vga-text-mode/#a-println-macro)

- [$crate変数](https://doc.rust-lang.org/1.30.0/book/first-edition/macros.html#the-variable-crate)
  - 自モジュール外では、`::foo`に展開 (`foo`は自モジュール名)
  - 自モジュール内では、何も展開されない (自モジュールの中なので展開の必要がない)
- `#[macro_export]` attribute
  - クレートのルートネームスペースに指定したマクロ配置する
    - 🙆‍♂️ `use crate::foo_macro`
    - 🙅‍♂️ `use crate::foo::foo_macro`
- `#[doc(hidden)]` attribute
  - `pub` として定義されていても生成されるドキュメンテーションからは隠される
  - マクロ内部で呼び出すメソッドなどに付けることが多い？
    - マクロは呼び出し元で展開されるため、本来なら隠蔽しておきたいメソッドなども`pub`としておく必要がある (自モジュール内なら不要)

### [Testing](https://os.phil-opp.com/testing/)

#### [I/O Ports](https://os.phil-opp.com/testing/#i-o-ports)

- x86におけるCPUと周辺機器の通信方法
  - Memory-mapped I/O
    - RAMを介さないメモリアクセスを通して通信する
  - Port-mapped I/O
    - それぞれ1つ以上のポートを持つ分離されたI/Oバスを利用
    - `in`, `out`と呼ばれる特別なCPU命令(instructions)で通信を行う
      - ポート番号とデータを取る

#### [Printing to the Console](https://os.phil-opp.com/testing/#printing-to-the-console)

- QEMUの出力をホスト側に送信する方法として[UART](https://ja.wikipedia.org/wiki/UART)を利用できる
  - [16550 UART](https://ja.wikipedia.org/wiki/16550_UART)は互換が豊富で`uart_16550`クレートもある
  
