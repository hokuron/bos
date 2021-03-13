# Writing an OS in Rust

## Learned Points

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
  
### [CPU Exceptions](https://os.phil-opp.com/cpu-exceptions/)

- interrupt descriptor table
  - CPU exceptionsに対応するためのハンドラー関数を提供
    - 無効なメモリーアクセスやゼロ除算など
- x86のCPU exceptionは約20種類
  - https://wiki.osdev.org/Exceptions
- タイプごとにハンドラー関数を呼び出す
  - ハンドラー関数を呼び出し中に例外が発生した場合の例外もある(**Double Fault**)
    - 目的のハンドラー関数が存在しなかったなど
  - Double Faultのハンドラー関数呼び出し中にさらに例外が発生した場合は何もできない(**Triple Fault**)
    - 自身をリセットしたりOSを再起動したりする

- [The Interrupt Descriptor Table (IDT)](https://os.phil-opp.com/cpu-exceptions/#the-interrupt-descriptor-table)
  - _segment_: Intelの用語で次のようなメモリー領域のことを言う
    - プログラムの実行中に使用され
    - ベースアドレス、サイズと
    - 実行や書き込みのアクセス権限が含まれる
  - _Global Descriptor Table (GDT)_: メモリー領域(_segments_)の特徴を定義したデータ構造体
    - x86系プロセッサーで使われている
    - 起源は[80286](https://en.wikipedia.org/wiki/80286)

  - IDTエントリー16バイトの構造体
    - ハンドラー関数へのポインターは3分割されている(なぜ？)
  - IDTインデックスは事前に決められている
    - ハードウェアは各種例外に対応するIDTエントリーを自動ロードする

#### [The Interrupt Calling Convention](https://os.phil-opp.com/cpu-exceptions/#the-interrupt-calling-convention)

- `extern "C" fn ...` (calling convention; 呼び出し規則)
  - x86_64 LinuxにおけるC言語の関数のルール([System V ABI](https://refspecs.linuxbase.org/elf/x86_64-abi-0.99.pdf))にRustは従っていない
  - `extern "C"`とすればそのルールに従うようになる

- Preserved(Callee-saved) registers
  - その値は関数呼び出しをまたがって変更しない
  - 呼び出し先(callee)は、呼び出し元に戻す前に元の値に復元する場合に限って上書きが可能
    - 関数の先頭でスタックに保存し最後に復元するのが一般的
  - x86_64: `rbp`, `rbx`, `rsp`, `r12`, `r13`, `r14`, `r15`
- Scratch(Caller-saved) registers
  - calleeは、その値を制限なく書き込み(上書き)可能
  - 呼び出し元(caller)が、関数呼び出し(callees)をまたがって値を保持する場合は、呼び出し前にスタックに保存する
  - x86_64: `rax`, `rcx`, `rdx`, `rsi`, `rdi`, `r8`, `r9`, `r10`, `r11`

- `extern x86-interrupt fn` calling convention
  - `x86_64` crateの`idt::HandlerFunc`
  - 割り込み専用？の呼び出し規則
  - 例外発生時に全レジスター(*)を保存し、関数(ハンドラーのこと？)から戻る際に元の値に復元される
    - 全レジスター(*)：コンパイラーは、効率化のため上書きされるレジスターのみバックアップする
    - 例外は任意の命令(instruction)で発生する
      - 対して通常の関数は、コンパイラーによって挿入された`call`命令により呼び出される
    - コンパイル時はどのコードが例外を発生されるか(ほとんどの場合)予知できない
      - 例外発生を予見してその直前にあらかじめレジスターの内容をバックアップすることもできない
      - 同じ理由で、例外ハンドラーでcaller-saved registerに依存する呼び出し規則を使用することもできない
  - 例外ハンドラーの引数は、スタック上の特定のオフセットから取り出す
    - [Interrupt Stack Frame](https://os.phil-opp.com/cpu-exceptions/#the-interrupt-stack-frame)で配置が決まっているから？
    - 通常の呼び出し規則では、引数はレジスターで渡されるが、`x86-interrupt`呼び出し規則では上書きが禁止されておりそれができない
  - リターン命令(instruction)は、通常の関数呼び出し時の`ret`ではなく`iretq`命令を使う
  - エラーコードの複雑な処理(*)を行う必要がある
    - 複雑な処理：例外ごとにエラーコードの有無があり、それによってスタックの配置(アライメント；alignment)が変わり、ハンドラーから戻る前にポップする必要がある
  - ただし、ハンドラーと例外の紐付けがないため、複雑な処理を行うべき関数かどうかは([Interrupt Stack Frame](https://os.phil-opp.com/cpu-exceptions/#the-interrupt-stack-frame)内の？)引数の数から推論する必要がある
    - `x86_64` crateの`InterruptDescriptorTable`は正しい紐付けを型安全な方法で保証してくれる
  
### [Double Fault](https://os.phil-opp.com/double-fault-exceptions/)

- 目的のハンドラーがスワップアウトされている場合でも発生しうる
  - 絶対ではない。代わりにPage Faultになりうる。詳細は後述
- エラーコードは常に`0`
- 返り値はなし([`Diverging function`](https://doc.rust-lang.org/stable/rust-by-example/fn/diverging.html))
- [AMD 64マニュアル](https://www.amd.com/system/files/TechDocs/24593.pdf)には正確な定義が載っている
  - > double fault exception _can_ occur when a second exception occurs during the handling of a prior (first) exception handler”
  - "_can_"が重要で、ごく限られた例外の組み合わせでdouble faultが発生する
    - [具体例](https://os.phil-opp.com/double-fault-exceptions/#causes-of-double-faults)
- 発生した例外に対するIDTエントリーがない(0個)場合、General Protection Faultが発生する
- さらにそれを定義していない場合、別のGeneral Protection Faultを呼び出そうとしたときにDouble Faultとなる
  - = 単純に、発生した例外に対するハンドラーがないからDouble Faultになる、というロジックではなく、特定の例外の組み合わせが呼び出された時に発生する

- guard page
  - スタックオーバーフローを感知するため、スタックの(最?)下部に配置される特殊なメモリーページ
  - ブートローダーが設定してくれる
  - 物理フレームにはマッピングされていないため、Page Faultが発生する
  - 発生時にinterrupt stack frameをそのスタックにプッシュすると、再びPage FaultとなりDouble Faultが発生する
  - それでもguard pageを指したままなのでThird Faultが発生してしまう
    - = 再起動されてしまう
  - Double Faultのスタックにはguard pageは存在しない(一般的にはどうなの？)
    - スタックオーバーフローは、そのスタック以下のメモリーを破壊するかもしれないから
  - スタックオーバーフローは、止まらない再帰呼び出しを行えば簡単に発生させられる
- x86_64アーキテクチャーは、正常なスタックに切り替える能力を持っている
  - ハードウェアレベル行われるため、CPUが例外スタックフレームをプッシュする前に切り替えられる
- Interrupt Stack Table (IST)
  - 切り替えの仕組みは、ISTとして実装されている
  - 7つの正常なスタックのポインターから成る
    - Double Faultは0番目()
    - x86はリトルエンディアンなので、トップアドレスを指定する
      - [コード上](https://os.phil-opp.com/double-fault-exceptions/#creating-a-tss)では、ポインターに`stack_end`を指定している
  - IDTエントリーの`stack_pointer`フィールドを介してISTのスタックを選択する
- Task State Segment (TSS)
  - 32-bit mode:
    - プロセッサーレジスターの状態などのタスクについての様々な情報の保持
    - [I/O許可ビットマップ(I/O port permissions bitmap)](https://ja.wikipedia.org/wiki/Task_state_segment#I/O許可ビットマップ) Stack Tableの保持
    - コンテキストスイッチ
  - 64-bit mode:
    - 32-bit modeと同じく[I/O許可ビットマップ(I/O port permissions bitmap)](https://ja.wikipedia.org/wiki/Task_state_segment#I/O許可ビットマップ) Stack Tableの保持
    - ISTの保持
    - Privilege Stack Tableの保持
      - privilege levelごとのスタックへのポインターから成る
      - CPUがユーザーモード中に例外が発生した際、カーネルモードに切り替えてから例外ハンドラーを呼び出す
      - その時にPrivilege Stack Tableの0番目のスタックに切り替える
    - コンテキストスイッチは持たない
        - 64-bit modeではサポート外
- Global Descriptor Table (GDT)
  - 64-bitモードでは主に以下の用途がある
    - カーネルスペースとユーザースペースの切り替え
    - TSS構造のロード
  - かつてはメモリーの[セグメント方式(memory segmentation)](https://ja.wikipedia.org/wiki/セグメント方式)で使われていた
    - ページングがデファクトになる以前のことで、プログラムを他から隔離するのに利用
- [末尾再帰#末尾呼出し最適化](https://ja.wikipedia.org/wiki/末尾再帰#末尾呼出し最適化)
  - 関数の最後が再帰呼び出しの場合、通常のループに変換する最適化方法
  - この変換により、スタックフレームの追加作成が行われずその使用量を一定に保てる
  - これを防ぐ(今回は意図的にスタックオーバーフローを発生させたい)には、[Volatile](https://docs.rs/volatile/0.2.6/volatile/struct.Volatile.html)型を利用する
- `allow(unconditional_recursion)`
  - 上記の最適化防止による無限ループ発生に対するコンパイラーの警告を抑える

### [Hardware Interrupts](https://os.phil-opp.com/hardware-interrupts/)

- キーボード入力の例
  - 定期的にカーネルがハードウェア(キーボード)に入力を確認させる(ポーリング)のではなく、
  - 入力される度にキーボードがカーネルにそれを通知する
- Interrupt Controller
  - 接続されたハードウェアを含む端末の割り込みを集約し、CPUに通知する
    - ハードウェアは直接CPUと繋げられないため、このInterrupt Controllerを通す
  - ほとんどのinterrupt controllerは、割り込みの優先レベルをサポートしている(programmable)
    - 正確性が求められるタイマーは、キーボードよりも優先されるなど
- ハードウェア割り込みは非同期で発生する

- Programmable Interrupt Controller (PIC)
  - [Intel 8259](https://ja.wikipedia.org/wiki/Intel_8259)
  - APIC (Advanced PIC)
    - 8259との互換性有り
  - APICより8259の方がセットアップが簡単
- 8259
  - 8本の割り込み(要求)ラインと、CPUとの通信用ラインを数本所有
  - プライマリー、セカンダリーの2つのPICを備える
    - プライマリーの割り込みラインの1つに、セカンダリーが接続される
    - | secondary | - | primary | - | CPU |
  - 割り込みライン15本(セカンダリー8本、プライマリー7本(1本はセカンダリーとの接続で使用))のほとんどは、割り当てを固定化されている
  - 各コントローラーは、I/Oポートを介して構成される
    - primary: 
      - `0x20` ("command" port) 
      - `0x21` ("data" port)
    - secondary:
      - `0xa0` ("command" port) 
      - `0xa1` ("data" port)
  - PICの割り込み番号は再マッピングする必要がある
    - デフォルトでは0~15の割り込み番号を送信する
      - 0~15は、CPU exceptionsに占有済み
    - 最初の空き番号である32から、47までの範囲が一般的に選択される
  - この設定は、command, data portsに特別な値を書き込むことで行われる
    - [`pic8259_simple`](https://docs.rs/crate/pic8259_simple/0.2.0/source/src/lib.rs) crateが代わりにやってくれる！
- End of Interrupt (EOI)
  - 割り込みは処理され、システムが次の割り込みの受け入れ可能になったことを示すシグナル
  - PICは、このシグナルをハンドラーから明示的に受け取る必要がある
  - セカンダリーPICから送信した割り込みに対するEOIシグナルは、セカンダリーが接続されているプライマリーPICにも届く

- timer interrupt
  - 実行中のプロセスを定期的に中断し制御をカーネルに戻す
  - その際にカーネルは別プロセスに切り替えることもできる
  - こうすることで複数のプロセスが同時に実行しているように見せかけられる
    - 高校時代に習った「高速ペロペロキャンディー」
- Programmable Interval Timer (PIT)
  - 次の割り込みまでの間隔を設定できる

- [x86_64::without_interrupts](https://docs.rs/x86_64/0.12.1/x86_64/instructions/interrupts/fn.without_interrupts.html)
  - 割り込みフリー(interrupt-free)な環境で実行されるクロージャーを提供する
    - Mutexロックが解放されない限り割り込みが発生しない
  - `vga_buffer::print`の実装にある`WRITER`をロック中に割り込みが発生し、その割り込みハンドラー(非同期実行)の中で`print!`を実行しようとするとデッドロックが発生する
  - 短時間だけ割り込みを無効にする場合には有効な手段
    - それ以外では、割り込み待ちに要する時間が長くなりシステムの(割り込みに対する)反応が悪く(長く)なってしまう

- Scancode
  - キーボードコントローラーが読み取った押されたキーの情報


### [Introduction to Paging](https://os.phil-opp.com/paging-introduction/)

- x86におけるメモリー保護は、segmentationとpagingの2種類

- Segmentation
  - Fragmentation(後述)が原因でx86の64-bitモードではサポートされていない機能
    - 代わりにPaging(後述)が使われ、Fragmentationへ対処している
  - かつて(1978-)は、アドレス指定可能なメモリ量を増やすためのものだった
  - CPUは16-bitのアドレスのみ使用していたため、アドレス指定できるメモリ量は64KiBに制限されていた
  - セグメントレジスター _群_ (_registers_)を追加し最大1MiBまでアクセスできるようにした
    - それぞれのレジスターにはオフセットアドレスが含まれ
    - CPUは各メモリーへのアクセスにこのオフセットを自動で加算していた
  - CPUは、メモリーアクセスの種類ごとにセグメントレジスターを選ぶ
    - `CS`(Code Segment): instructionのフェッチ
    - `SS`(Stack Segment): プッシュ・ポップのスタック操作
    - `DS`(Data Segment), `ES`(Extra Segment): その他のinstructions
    - `FS`(Free?), `GS`(General?): 自由に使える(なんの略かは記載なし)
      - のちに追加された
  - 保護モード([Protected Mode](https://en.wikipedia.org/wiki/X86_memory_segmentation#Protected_mode))
    - segment descriptorにlocal or global descriptor tableへのインデックスが含まれる
      - オフセットアドレスとセグメントのサイズ、アクセスパーミッションが含まれるテーブル
    - プロセスごとに別々のlocal/global descriptor tableをロードすることで、OSはプロセスを相互に分離できる
      - プロセスは、メモリーアクセスできる範囲を自身のメモリー領域に制限される
  - 実際のメモリーアクセス前にアドレスを変更する = 仮想メモリー

- [Virtual Memory](https://os.phil-opp.com/paging-introduction/#virtual-memory)
  - ストレージからメモリーアドレスを抽象化する
  - ストレージに間接的にアクセスするため、 選択中のセグメントのオフセットアドレスを加算する変換ステップが最初に行われる
  - _virtual(仮想)_: 変換前の呼び名で、そのアドレスは変換機能によって異なる
    - 異なるアドレスが同じ物理アドレスを指す場合もあれば、同じアドレスが異なる物理アドレスを指すこともある(変換機能依存)
  - _physical(物理)_: 変換後の呼び名で、そのアドレスは一意でメモリー位置ごとに常に同じ場所を参照する
  - OSは、プログラムの再コンパイルなしに利用可能なメモリーをフルに利用できる
    - プログラムが異なる仮想アドレスを使用していても、物理メモリーの位置は任意の場所に配置できる(同じ場所でも異なる場所でも両方可能)
- [Fragmentation](https://os.phil-opp.com/paging-introduction/#fragmentation)
  - 物理メモリーが歯抜けの状態
  - 仮想メモリーと同じサイズを物理メモリー上に _連続して_ 確保できない
  - オフセットを詰めれば歯抜けは解消できる
    - プログラムの実行を一時中断したり物理メモリーの大量コピーが発生したり、それを定期的に実行したりとパフォーマンス劣化につながる

- [Paging](https://os.phil-opp.com/paging-introduction/#paging)
  - 仮想・物理メモリー空間の両方を固定サイズの小さなブロックに分割
  - Page: 仮想メモリー空間のブロック
  - Frame: 物理アドレス空間のブロック
  - 各pageは個別にframeに紐付けられるため、大きなメモリー領域を非連続的な物理フレームに分割できる
    - デフラグ不要
  - frameは、すべて同じサイズで、このサイズより小さいサイズで使われることはなく、segmentationのような断片化は起き得ない
- Internal Fragmentation ([Hidden Fragmentation](https://os.phil-opp.com/paging-introduction/#hidden-fragmentation))
  - 全てのメモリー領域が確実にpageサイズの倍数となるわけではない
    - size 50 per page; size 101 -> 3 pages; size 150, remainder size 49
  - internal fragmentationはサイズを無駄にするものの、デフラグはする必要がなく、segmentationのそれ(_External Fragmentation_ という)とは違い断片化量は*予測可能なのでまだマシ
    - *予測可能：メモリー領域あたり平均して半分のpageが断片化する
- [Page Table](https://os.phil-opp.com/paging-introduction/#page-tables)
  - pageとframeの紐付け情報を保持するテーブル
  - プログラムごとに自身のpage tableを持つ
  - `CR3`レジスター：↑からどのpage tableが選択されているかを保持しているレジスターで、`x86`においては`CR3`となっている
    - 保持しているのは、page tableへのポインター
  - OSは、プログラム開始前に`CR3`をロードし、page tableのポインターを得る
  - tableポインターの読み込み -> pageから紐付けられたframeを探索 -> メモリーアクセスを効率良くするため、多くのCPUではこの変換結果をキャッシュする
    - キャッシュ方法によってはパーミッションも保持する
- [Multilevel Page Tables](https://os.phil-opp.com/paging-introduction/#multilevel-page-tables)
  - (たぶんpageの)アドレス領域ごとに異なるpage tablesを使用する
    - 少数のpageと対応するframeであっても、pageが100万単位の歯抜け状態ではpage tableが巨大になってしまう
      - pageとテーブルのインデックスが一致しているとは限らないため、CPUは直接目的のエントリーに飛べない
  - level 2: 追加されたpage tableで、アドレス領域ごとに対応するlevel 1を間接的(*)に持つ
    - 間接的(*): 次のレベル(level 1)のテーブルが格納？されている物理メモリ(frame)と紐付けれる
  - level 1: 前述までのpageとframeを紐付けたpage table
    - level 2のアドレス領域は引き継がないためオフセットは持たない
    - (必ず？)page "0"から始まる
    - frameは物理アドレスを示す(仮想アドレスだと再起的に変換が繰り返されてしまう)
  - 歯抜け分のlevel 1 page tablesが無くなるため、tableを構築するために必要なメモリー利用が節約できる
  - level 2, level 1構成は、**two-level page table**という
  - より多くのlevelを増やすことも可能
  - `CR3`のようなpage tableを指すレジスターは、もっとも高いレベルのテーブルを参照している
  - 各レベルのテーブルは、次の下位レベルのテーブルを指していく
  - _multilevel_ または _hierarchical page table_

#### [Paging on x86_64](https://os.phil-opp.com/paging-introduction/#paging-on-x86-64)

- 4-levels構成
- pageサイズは4KiB
- 各page tableのエントリー数は512個で固定
  - 1エントリー8バイト； 512 * 8 = 4KiB
- page tableのインデックスは仮想アドレス(リンク先の画像参照)から直接取得できる
  - インデックスは9ビットで構成され、12-48ビットの間にレベル1-4の順で配置
    - 2^9=512となりエントリー数の512個と一致する
  - 0-12ビットはpageのオフセット
    - TODO: "2^12 bytes = 4KiB"の謎(12-bitなのに急にbytesが登場)を解く
    - 変換過程の最後、level 1から最終的なframeを発見後、そのframeにオフセットを加算し物理アドレスを導き出す
  - 48-64は破棄され、47ビット目のコピーとなっており5-level目のサポートを見越して予約されている
    - この構成が2の補数に似ていることから _sign-extension_ と呼ばれる
    - sign-extensionになっていなければCPUは例外を投げる
    - x86_64は実際には48ビットまでしかサポートしてない
      - "Ice Lake"(以降？)ではオプションで5-level目のサポートをした
- x86_64(64-bit mode)では、4-level paging階層が強制される
  - ブートローダーを設定した時点で達成できている
    - カーネルのpageとframeの紐付けや、正しいアクセス権限の設定

- [Page Table Format](https://os.phil-opp.com/paging-introduction/#page-table-format)
  - `x86_64` crateは、[PageTable](https://docs.rs/x86_64/0.12.1/x86_64/structures/paging/page_table/struct.PageTable.html)と[PageTableEntry](https://docs.rs/x86_64/0.12.1/x86_64/structures/paging/page_table/struct.PageTableEntry.html)を提供してくれる
  - エントリーの構成は、12-51ビットが物理メモリーの格納場所でframeか次レベルのpage tableを指す
  - それ以外はフラグまたはOSが自由に使えるようになっている
  - bit 0; `present`: 紐付けされたpageとされていないpageを区別する
    - ページを一時的にスワッピング(swap out)するのに使用
    - その状態でページがアクセスされるとpage fault例外が発生し、OSがページを再ロード(swap in?)しプログラムが続行される
  - bit 1; `writable`, bit 63; `no executable`: 読んで字の如く
  - bit 5; `accessed`, bit 6; `dirty`: pageに対して読み書き発生時にCPUが自動でセットするフラグ
    - どのページをスワッピング(swap-out)するか、ページを保存以降に変更されたかなどOSが活用できる情報となる
  - bit 3; `write through caching`, bit 4; `disable cached`: 個々のpageのキャッシュ制御
  - bit 2; `user accessible`: ユーザー空間のコードからpageが利用可能かのフラグで、許可されていなければ、CPUがカーネルモードの場合にのみアクセスできる
    - システムコールを高速化できる
      - ユーザー空間のプログラムを実行中にカーネルマッピング(kernel mapped)を維持する(切り替え不要ってこと？)
    - [Spectre](https://ja.wikipedia.org/wiki/Spectre)の脆弱性で、ユーザー空間のプログラムからもページが見れるようになってしまった..
  - bit 8; `global`: pageが全アドレス空間で利用可能で、アドレス空間スイッチの変換キャッシュから削除不要("Translation Lookaside Buffer"で後述)であることをハードウェアに通知する
    - 通常、カーネルのコードを全アドレス空間と紐付けするために使われる
      - 許可された(cleared)`user accessible`と一緒に利用される
  - bit 8; `huge page`: より大きなサイズのpagesを作成できるかのフラグ
    - これは、level 2またはlevel 3のpage tablesが、紐付けられたframeを直接指すようにすることで可能にする
    - pageサイズは、512倍に増えlevel 2, 3のそれぞれのエントリーは2MiB(512 * 4KiB)、1GiB(512 * 2MiB)になる
    - 変換キャッシュの行数および必要とするpage tablesを削減できる

- [Translation Lookaside Buffer](https://os.phil-opp.com/paging-introduction/#the-translation-lookaside-buffer)
  - page tableの最後の数個の変換結果をキャッシュし、ヒットした変換をスキップする仕組み
    - 4つのレベルの変換は、それぞれメモリーアクセスが必要で高価になってしまうため
  - `invlpg` (invalidate page) instruction: 特別なCPU命令で、TLB中の指定したpageの変換(のキャッシュ？)を削除し、次回アクセス時にpage tableから再ロードさせる：
    1. 他のCPUキャッシュと違い、page tableの内容が変わっても(キャッシュした？)変換の更新や削除は行わないため
    2. カーネルがpage tableを変更するたびに、カーネルはまた、TLBを手動で更新する必要があるため
  - `CR3`レジスターをリロードすることでバッファーに書き込む(flush)ことも可能
    - (復習)`CR3`レジスターは、アドレス空間を切り替える
    - `x86_64` crateでは`tld`モジュールが存在する

- `CR2`レジスター：page fault発生時にCPUが自動でセットする
  - 発生時にアクセスした仮想アドレスを保持する

### [Paging Implementation](https://os.phil-opp.com/paging-implementation/)

#### [Accessing Page Tables](https://os.phil-opp.com/paging-implementation/#accessing-page-tables)

- page tableの各エントリーは、次点のテーブルの _物理_ アドレスを保持している
  - さもないとアドレス変換の無限ループに陥ってしまう
- 特定のアドレスにアクセスする場合、それは、page tableに格納される物理アドレスにアクセするのではなく、仮想アドレスにアクセスすることになる
- 物理アドレスへのアクセスは、それに紐づけられている仮想アドレスを介すしかない
  - ブートローダーがpage table階層を設定しているため、カーネルも仮想アドレス上で動作し、物理アドレスへの直接アクセスができない
- いくつかのvirtual pageをphysical page frameに紐付けることで、任意のpage table frameにアクセスできるようにする

- `bootloader` crateの`entry_point`マクロを使えばカーネルのエントリーポイントである`_start()`メソッドの代わりとなる任意の関数を指定できる
  - `extern "C"`や`[no_mangle]`も不要になる

- `unsafe fn`の中身は、`unsafe`ブロックで囲む必要がない([RFC](https://github.com/rust-lang/rfcs/pull/2585))
  - 関数本体全体を`unsafe`ブロックで囲んだ場合と同等の扱いとなる
- 実際は`unsafe`ブロックを使わない、セマンティック的に安全でない操作(メモリーの直操作など)を表現した`unsafe fn`の場合、本当に`unsafe`な操作が紛れ込んでも気付きにくい
- そのような`unsafe fn`内部では、すぐさま通常の`unsafe`ではないプライベートな`fn`を呼び出すことでこの問題を避けられる

```rust
pub unsafe fn unsafe_modify() {
    no_need_unsafe_block_modify()
}

fn no_need_unsafe_block_modify() {
    // no `unsafe` blocks
    // ...
    // ...
}
```

### [Heap Allocation](https://os.phil-opp.com/heap-allocation/)

- Local Variable
  - ローカル変数は、[コールスタック](https://ja.wikipedia.org/wiki/コールスタック)(スタック構造；`push`と`pop`操作可能)上に格納される
  - 呼び出された関数の引数、戻り(値？)アドレスとローカル変数は、コンパイラーがプッシュする
- Static Variable
  - スタック("the stack"; コールスタックを指している？)から分離された固定のメモリーロケーションに格納される
    - 固定のメモリーロケーション：リンカーがコンパイル時に割り当て、実行可能形式にエンコードされる
  - コンパイル時に格納場所が判明しているため、アクセスの際に参照行為は不要となる
    - 静的変数へのアクセスに静的変数を指す別の変数は導入せずとも、直接アクセスが可能
  
#### [Dynamic Memory](https://os.phil-opp.com/heap-allocation/#dynamic-memory)

- ローカル変数も静的変数もそのサイズは固定である必要がある
  - 動的に要素が追加され増大するコレクション系の値は直接格納できない
  - 条件付きでそれを可能にする提案がなされている [RFC #1909: Unsized Rvalues](https://github.com/rust-lang/rust/issues/48055)
- 解決策：**heap**; _dynamic memory allocation_
  - `allocate`: 指定したサイズのメモリーの空きチャンクを返す。変数はそこに格納する
    - 指定サイズのメモリーブロックを確保し、`*mut`のような生ポインターを返す
  - `deallocate`: 格納した変数の参照を使って、↑を開放する。変数の寿命もそこまで

#### [The Allocator Interface](https://os.phil-opp.com/heap-allocation/#the-globalalloc-trait)

- [GlobalAlloc](https://doc.rust-lang.org/alloc/alloc/trait.GlobalAlloc.html) trait
  - allocationとその収集が必要な場所に、コンパイラーがこのtraitのメソッドの呼び出しコードを挿入する
    - プログラマーが明示的に利用することはない
  - `alloc`メソッド：[`Layout`](https://doc.rust-lang.org/alloc/alloc/struct.Layout.html)インスタンス(サイズとアライメント)を引数に取り、割り当てを行ったメモリーブロックの1バイト目の生ポインター(`*mut u8`)を、または、エラー時にNULLポインターを返す
  - `dealloc`メソッド：`alloc`の返却値とそれに渡された引数`Layout`を引数に取る
  - `alloc_zeroed`メソッド：`alloc`を呼び出し、割り当てメモリーブロックをゼロ化する
    - デフォルト実装あり
  - `realloc`メソッド：割り当てを増やしたり減らしたりする
    - デフォルト実装では、指定されたサイズの新たなメモリーブロックを割り当て、元の割り当て内容をコピーする
  - trait自身と各メソッドは、`unsafe`として定義されている
    - 前者は、実装者が正しく実装していることを保証する必要があるため
      - 未使用で有効なメモリーブロックを割り当てるなど
    - 後者は、呼び出し側がさまざまな不変条件を保証する必要があるため
      - `Layout`のサイズが0はダメなど
      - ただ、呼び出し元はコンパイラーとなるため、これらの要件は確実に満たされる
- [`#[global_allocator]` Attribute](https://os.phil-opp.com/heap-allocation/#the-global-allocator-attribute)
  - アロケーターインスタンスがどれをコンパイラーに伝える
  - `static`変数にする必要がある
- [`alloc_error_handler` Attribute](https://os.phil-opp.com/heap-allocation/#the-alloc-error-handler-attribute)
  - `alloc`メソッドのエラーを表すぬるぽを返した際に呼び出される関数を指定する
    - 引数は、`alloc`に渡された`Layout`インスタンスが渡ってくる
  - 安定化しておらずfeature gateが必要

#### [Creating a Kernel Heap](https://os.phil-opp.com/heap-allocation/#creating-a-kernel-heap)

1. 任意の(まだ使われていない)仮想メモリー領域を決める
  - ヒープの開始アドレスからヒープサイズを足したアドレスまで
    - 0スタートなので最後に1を引いたのがヒープの終端アドレス
  - ヒープアドレスの開始と終了をそれぞれ`Page::containing_address()`で`Page`型に変換
  - `Page::range_inclusive()`でpage領域を作成
1. その領域を物理メモリーと紐付ける
  - page領域をイテレートして個別のpageに分解
  - pageと紐づける物理frameを確保し (`FrameAllocator::allocate_frame()`)
    - 残りフレームがなければエラーとする (`MapToError::FrameAllocatorFailed`)
  - pageに読み書きのフラグをセット (`PageTableFlags::PRESENT | WRITABLE`)
    - これらのアクセス許可はヒープメモリーにとって重要
  - pageとframeを紐づける (`Mapper.map_to()`)
  - Translation Lookaside Buffer(TLB)に書き込む (`MapperFlush.flush()`)

### [Allocator Designs](https://os.phil-opp.com/allocator-designs/)

カーネルコードのallocationパターンは、ユーザースペースのそれと比べると単純。

#### [Bump Allocator](https://os.phil-opp.com/allocator-designs/#bump-allocator)

- _Stack Allocator_ とも言う
- もっとも単純なデザイン
- メモリーを線状に割り当て、割り当てたバイト数と割り当て数を追跡する
- メモリーの解放は、1回ですべて行うことしかできないため、特定のユースケースでのみ有効なデザインとなっている
- `next`変数：未使用のメモリーの先頭を指す(ヒープの先頭アドレスを指すところから始まる)
  - 割り当てを行うごとに増加していく(increasing; _bumping_; 進ませる；移す)
  - ヒープの最後のアドレスに向かって一方向に増加し、割り当て不可状態になった後、割り当てを行おうとすると、out-of-memoryエラーになる
- 割り当てカウンターを持ち、それが0(すべての割り当てが解放されたことを意味する)になると`next`変数は再びヒープの先頭アドレスを指す
  - `alloc`で1増加、`dealloc`で1減少する
- 早い
  - アセンブリレベルの数個の命令へと最適化される
  - virtual DOMライブラリーにも利用されている
- global allocatorとして使われることはあまりなく、原理がarena allocatorの形で適用されることが多い
  - arena allocatorは、個々のallocatorをまとめる
- 全割り当てが解放されるまでメモリの再利用ができない
  - 割り当て後すぐに解放されても`next`の位置を戻さない(戻せない)


- [`GlobalAlloc` and Mutability](https://os.phil-opp.com/allocator-designs/#globalalloc-and-mutability)
  - `GlobalAlloc` traitの必須メソッドは、_nonmutating_ (`&self`)
  - `#[global_allocator]` attributeで指定する変数は、`static`にする必要あるため、_mutating_ (`&mut self`) なメソッドじゃない
  - `spin::Mutex`(内部可変性；interior mutability)を利用する

#### [Linked List Allocator](https://os.phil-opp.com/allocator-designs/#linked-list-allocator)

- 別名 _Pool Allocator_
- メモリーの空き領域自体に解放された領域の情報を持たせる
  - 情報：その空き領域のサイズおよび次の空き領域への(先頭)ポインター
- 解放領域追跡に関する追加のメモリーは必要無くなるため、無限の追跡が可能になる
- パフォーマス面ではBump Allocatorより劣る
- `head`: 最初の未使用領域のポインターのことを言う
- [`free list`](https://en.wikipedia.org/wiki/Free_list): すべての未使用領域を追跡した構造体(linked list)を言う
- Bump Allocatorよりも汎用的
- 隣接する解放された領域のマージが必要になる
  - 割り当てと解放を繰り返すごとに解放メモリー領域がフラグメントを起こす
  - 細かい領域へと分断されていくため、比較的小さいサイズのメモリーも割り当てられなくなりエラー(`mem::null_mut()`返却)になってしまう
- `linked_list_allocator` crateでは、解放メモリー領域の開始アドレスでソートされた状態のリストを保持し、deallocate時に直接マージすることで断片化しないようにしている
  - タイミングはどうであれ、マージ処理が必要になることには代わりない
- 未使用のメモリー領域の数に比例してリストが長くなるため、パフォーマンスの良し悪しは、ヒープをどれぐらい利用するか(のプログラム)に依存する
  - Linked List Allocator自身ではなく、それを利用するプログラムによって良し悪しが決まってくる

#### [Fixed-Size Block Allocator](https://os.phil-opp.com/allocator-designs/#fixed-size-block-allocator)

- 固定サイズ故、必要なサイズ以上のメモリー領域を確保することがある(internal fragmentation)
- Linked List Allocatorと比べて、ちょうどいいサイズの空き領域を探す時間を劇的に減らせる
- 何パターンかの固定サイズの領域を用意しておき、確保時にはその中から要求メモリーサイズに収まる領域を使う
- 未使用メモリーはLinked List Allocatorと同じようにlinked list形式で追跡する
- ただし、サイズごとに個別のリストを作成するため、各リストには単一サイズの領域だけが格納される
  - 単一の`head`ではなく、`head_16`, `head_512`などのようにサイズごとに専用の`head`が存在する
  - 各リストからサイズ情報を取り除ける(次の要素へのポインターだけが残る)
- どの`head`を使うか決まれば、そのリストの最初の要素を割り当てるだけなので、Linked List Allocatorのように走査が不要で速い
- サイズパターンを2の累乗にするとことで、最悪の場合で割り当てサイズの半分、平均して1/4に割り当てメモリーの浪費を制限できる
  - この場合、各パターンのアライメントとサイズと同じになる
- カーネルは、大きめ(2KB以上)のサイズを割り当てることが稀なため、それ専用の別のallocator(fallback allocator)を用意しメモリー浪費を減らす
- `dealloc`時もどのパターンのサイズに収まるか繰り上げ計算が必要
  - コンパイラーは、`alloc`時に返却したサイズではなく、引数で指定してきたサイズ情報を再び`dealloc`時にも渡してくる
- あるパターンサイズの領域が枯渇した場合：
  1. 用意していたfallback allocatorを利用する
  1. 枯渇したパターンより大きいパターンを分割する
      - 16-byteが枯渇したなら、32-byteを分割し16-byteを2つ増やす(32-byteは1つ減る)
  - 前者のほうが実装が単純
- Linked Listよりも速くメモリー浪費が発生するが、カーネルのようなパフォーマンス重視であれば、Fixed-Sizeはより良い選択になる

- [Slab Allocator](https://en.wikipedia.org/wiki/Slab_allocation)
  - 選択された型専用のメモリー領域を直接割り当てる方法
  - 要求された型のサイズとピッタリ一致するため、メモリー浪費がない
  - 型によっては未使用領域を事前に初期化し、あらかじめその型のインスタンスを保持しておくことも考えられる
  - 他のallocatorと一緒に使われることが多い
- [Buddy Allocator](https://en.wikipedia.org/wiki/Buddy_memory_allocation)
  - 未使用領域の追跡にbinary tree(2の累乗のブロックサイズ)を利用した方法
  - 要求サイズによってはブロックを2分割し、解放時には隣接するブロックが未使用になると再び結合される
  - external fragmentationが減り、小さなブロック群を大きな要求サイズに割り当てられるようになる
  - fallback allocator要らずで、パフォーマンスが予測可能になる
  - internal fragmentationは防げず、メモリー浪費が発生する
  - 割り当て領域をさらに小さく分割するため、Slab Allocatorと組み合わせる場合もある

### [Async/Await](https://os.phil-opp.com/async-await/)

- [Preemptive Multitasking](https://os.phil-opp.com/async-await/#preemptive-multitasking)
  - OSの機能(割り込み処理)を利用して任意の時点でスレッドを切り替える
    - そのためにスレッドに一時停止を強制させる
  - マウスやパケット到着、タイマーなどの割り込みの度にCPUの制御を奪い(regain)、割り込みハンドラーでタスクを切り替える
    - 最初の2つの割り込みは受動的で、最後のタイマーは、意図的に仕込めば能動的に割り込みのタイミングを設定できる
  - [Saving State](https://os.phil-opp.com/async-await/#saving-state)
    - タスク切り替え時に処理が中断されたタスクのすべての状態は、OSがバックアップし処理再開に備える ([_context switch_](https://ja.wikipedia.org/wiki/コンテキストスイッチ)という)
      - 状態にはcall stackとCPUレジスターのすべての値が含まれる
    - しかし、call stackは巨大になり得るため、call stackの内容をバックアップする代わりに、タスクごとに個別のcall stackを設定する (このタスクをお馴染み[_thread_](https://ja.wikipedia.org/wiki/スレッド_(コンピュータ))と言う)
    - 個別スタックを用いることで、context switchへはCPUレジスターの値のみ保持させるだけで済む
    - 最大で毎秒100回発生するcontext switchのオーバーヘッドを最小化できる
  - メリット
    - OSがタスクの実行時間を完全に制御できるため、各タスクはCPUタイムを公平に共有可能(fair share)
    - 第三者のタスクや複数ユーザーで共有されるシステムにおいては重要事項
  - デメリット
    - 各タスクごとにcall stackが必要になるため、メモリー使用量が多くなったり、タスク数に制限がかかることが多い(共有call stack比)
    - タスク切り替えの度に、未使用のCPUレジスターを含めてすべての状態を保存する必要がある
  - Preemptive MultitaskingとThreadは、OSの基本的なコンポーネント
    - 信頼できないユーザー空間のプログラムを実行するためにも必要

- [Cooperative Multitasking](https://os.phil-opp.com/async-await/#cooperative-multitasking)
  - タスクがCPUの制御を定期的に放棄する必要がある
  - Preemptive Multitaskingとは異なり、タスクが自主的に都合の良いタイミング(例えばI/O待ち中)でCPUの制御を放棄する
  - coroutineやasync/awaitなどプログラミング言語レベルで用いられる
    - プラグラマーかコンパイラーが[`yield`](https://en.wikipedia.org/wiki/Yield_(multithreading))(CPUの制御を放棄し別のタスクを実行させる)をプログラム中に挿入する
  - 非同期操作([非同期I/O](https://ja.wikipedia.org/wiki/非同期IO)など)と組み合わせて利用するのが一般的
    - 非同期操作は、その操作が完了していない場合"no ready"状態を返却する
    - この時、↑の完了を待機しているタスクは、`yield`を実行して他のタスクを実行させられる
  - [Saving State](https://os.phil-opp.com/async-await/#saving-state-1)
    - タスクは、いつ停止するか自身で決めるためOSに頼らずに、実行再開時に必要な状態を過不足なく停止前に保存する
    - これによりパフォーマスに優れている
    - Rustなどは、call stackの必要な部分をバックアップする
      - 必要なすべてのローカル変数を自動生成された構造体に格納する
    - すべてのタスクで単一のcall stackを共有できるため、メモリー使用量が大幅に削減できる
      - Preemptive Multitaskingとは違い、任意の数のタスクが作成可能になる
  - デメリット
    - uncooperativeなタスクが制限なく実行される可能性がある
      - 悪意のあるタスク、バグってるタスクは、他のタスク実行を妨げ、システム全体に影響を与える恐れがある
    - すべてのタスクがcooperativeであることが必要(そうでないならCooperative Multitaskingは使うべきではない)
    - OSは、任意のユーザーレベルのプログラムに依存させるのは避けるべき
  - メリット
    - 強力なパフォーマスとメモリーの利点は、非同期操作との組み合わせが最適
    - カーネルは、パフォーマスが重要なプログラムなので、Cooperative Multitaskingは非同期ハードウェアと相互作用させるのに優れた方法となる

#### [Async/Await in Rust](https://os.phil-opp.com/async-await/#async-await-in-rust)

- `future`
  - `poll`メソッドは、`Pin<&mut Self>`(`self:`)と`&mut Context`(`Waker`を内包)を引数にとる
- コンパイラーは、`async`関数本体をstate machineに変換する
  - `.await`の呼び出しそれぞれで異なる状態(state)を表現する
  - このstate machineは、`Future` traitを実装する
  - "Start"から"End"まで状態の変遷が起きる
    - その間の状態は`.await`呼び出しの数に依存する
  - "End"に達すると`poll`メソッドで、`Poll::Ready`(結果を内包するvariant)を返す
  - それまでは待ち状態(`.await`呼び出しごとにつくれらる状態)に遷移し`Poll::Pending`を返す
  - 次の`poll`メソッド呼び出し時に最後の待ち状態から再開する
  - ↑これを実現するためにstate machineは、
    - 内部で現在の状態を追跡し続けたり
    - 再開する処理で使う変数(ローカル変数や実引数)を保存したりする必要がある
  - 保存される変数は、stateに紐付けられる構造体のフィールドとして変換される
    - state一つひとつはenumのvariantで、各構造体はそれらに内包される
    - 両者ともコンパラーが自動で生成する
  - `future`そのものは、最初の`poll`メソッドが呼び出されるまで何もしない
- [Pinning](https://os.phil-opp.com/async-await/#pinning)
  - [Self-Referential Structs](https://os.phil-opp.com/async-await/#self-referential-structs)
    - 自身のフィールドの一部を参照するポインターを持つ場合もself-referential structとなる
    - moveした際、参照先のアドレスが変わっても、そのポインターが新しいアドレスに追随していない状態になる
    - pinningはこのmoveを禁止する
  - `Box`でラップしてヒープに収めても、`mem::replace`や`mem::swap`できるので、同じアドレスに固定しておくのは難しい
  - `Pin`はラッパータイプ、`Unpin`はtrait (auto trait)
  - `Pin`がラッピングしている値を`&mut`として安全に取り出すには、その値の型が`Unpin`である必要がある
  - self-referential structから`Unpin`をopt-outすれば、moveによってアドレスが変わらないことが保証される
    - `PhantomPinned`型のフィールドを用意するとout-outされる
  - async/awaitで生成された`Future`インスタンスは、self-referentialになることがあるため、`poll`メソッドのレシーバーは`self: Pin<&mut Self>`となる
  - 加えて、`poll`メソッドの呼び出しの間(中)にmoveしないよう`Unpin`はout-outされる
- [Executors](https://os.phil-opp.com/async-await/#executors)
  - 独立したtasksとしてfuturesをスポーンする([spawn](https://en.wikipedia.org/wiki/Spawn_(computing)))
  - 全futuresが完了するまでポーリングする
  - 全futuresを集約することで、`Poll::Pending`が返された際に別のfutureに切り替えることが可能になる
    - 並列(parallel)で処理できCPUを使い続けられる(the CPU is kept busy)
  - スレッドプールを作成し複数のCPUコアを利用する
  - [work stealing](https://en.wikipedia.org/wiki/Work_stealing)(手持ち無沙汰になったら他のコアのキューに積まれたタスクを奪う)を活用してコア間の負荷を分散する
  - レイテンシーとメモリーオーバーヘッドが少ないシステムに組み込みのexecutorもある
  - ポーリングを反復するオーバーヘッドを避けるために、Rustのfutureでサポートされている _waker_ APIも利用する
- [Wakers](https://os.phil-opp.com/async-await/#wakers)
  - [`Waker`](https://doc.rust-lang.org/nightly/core/task/struct.Waker.html)は、`poll`メソッドの引数で渡される[`Context`](https://doc.rust-lang.org/nightly/core/task/struct.Context.html)にラップされる
  - executorが作成し、非同期タスクが完了(一部完了)を通知するのに使われる
  - executorは、`poll`呼び出し時に`Poll::Pending`が返されたら、対応するwakerから通知されるまでは再び繰り返し`poll`を呼ばなくて済む
- [ExecutorsとWakersはCooperative Multitasking](https://os.phil-opp.com/async-await/#cooperative-multitasking-1)
  - executorのtaskは、cooperative task
  - futureは、`Poll::Pending`を返却することでCPUの制御を放棄している
    - やろうと思えば`Poll::Pending`を返さずに処理を実行し続けられる
  - futureは、次の`poll`呼び出し上で実行を続けるのに必要なすべてのstateを格納している
  - async/awaitでは、↑で必要なすべての変数を検出し、生成したstate machine内に格納する
    - cooperative multitaskingが必要な状態を自身で保存するのと一緒の挙動

