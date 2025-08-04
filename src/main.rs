use ctor::ctor;
use lazy_static::lazy_static;
use std::fs::File;
use std::io::{Error, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use libc;

// 全局原子变量，用于跨线程传递信号状态
lazy_static! {
    static ref TERM_FLAG: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
}

// 堆栈保存函数：在正常执行流中调用，安全执行复杂操作
fn save_stack_trace() -> () {
    let mut file = File::create("train_stack_trace.txt").unwrap();
    let mut backtrace = String::new();

    backtrace::trace(|frame| {
        backtrace::resolve_frame(frame, |symbol| {
            if let Some(name) = symbol.name() {
                backtrace.push_str(&format!("{:?}\n", name));
            }
            if let Some(file) = symbol.filename() {
                if let Some(line) = symbol.lineno() {
                    backtrace.push_str(&format!("  at {}:{}\n", file.display(), line));
                }
            }
        });
        true // 继续捕获所有帧
    });

    writeln!(file, "Model training crashed. Stack trace:\n{}", backtrace).unwrap();
    eprintln!("堆栈信息已保存到 train_stack_trace.txt");
}

// 信号监听线程：低消耗轮询
fn signal_listener() {
    // 注册需要监听的异常信号（根据训练需求调整）
    let signals = [
        signal_hook::consts::SIGTERM, // kill 命令
        signal_hook::consts::SIGINT,  // Ctrl+C
        signal_hook::consts::SIGABRT, // 程序异常终止
    ];

    // 为每个信号注册处理（设置TERM_FLAG为true）
    let registrations: Vec<_> = signals
        .iter()
        .map(|&sig| {
            signal_hook::flag::register(sig, Arc::clone(&TERM_FLAG))
                .expect("Failed to register signal")
        })
        .collect();

    // 低消耗轮询：每次检查后休眠100ms，平衡响应速度和资源消耗
    while !TERM_FLAG.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(100));
    }
    
    let pid = nix::unistd::getpid().as_raw(); // PID of the current process (thread group ID)
    let tid = pid;
    let ret = unsafe { libc::syscall(libc::SYS_tgkill, pid, tid, libc::SIGUSR2) };
    println!("Received termination signal, saving stack trace...");

    // 下面拿到的 当前线程（信号处理线程） 的堆栈，而非执行 main() 函数的主线程。
    // let _ = save_stack_trace();

    // 可选：执行其他清理操作（如保存模型中间状态）
    // save_checkpoint();

    // 退出进程（确保训练进程终止）
    thread::sleep(Duration::from_secs(10));
    eprintln!("训练进程已终止，堆栈信息maybe已保存。");
    std::process::exit(1);
}

// 程序启动时自动执行（通过ctor宏）
#[ctor]
fn init_signal_handler() {
    // 启动信号监听线程（与训练主线程分离）
    thread::spawn(|| {
        signal_listener();
    });
    eprintln!("信号监听已启动，将在异常时保存堆栈信息");
}

pub fn register_signal_handler<F>(sig: std::ffi::c_int, handler: F)
where
    F: Fn() + Sync + Send + 'static,
{
    unsafe {
        match signal_hook_registry::register_unchecked(sig, move |_: &_| handler()) {
            Ok(_) => {
                log::debug!("Registered signal handler for signal {sig}");
            }
            Err(e) => log::error!("Failed to register signal handler: {e}"),
        }
    };
}

#[ctor]
fn setup() {
    register_signal_handler(
        nix::libc::SIGUSR2,
        save_stack_trace,
    );
}


fn main() {
    // 模拟训练过程（实际训练逻辑替换为具体实现）
    eprintln!("开始模型训练...");

    // 模拟长时间运行的训练任务
    for i in 0..100 {
        thread::sleep(Duration::from_secs(1));
        println!("训练进度：{}%", i + 1);
    }

    eprintln!("模型训练完成");
}
    