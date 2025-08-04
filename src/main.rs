use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::io::Error;

// 保存堆栈到文件
fn save_stack_trace() -> Result<(), Error> {
    let mut file = File::create("stack_trace.txt")?;
    let mut backtrace = String::new();
    
    // 使用 backtrace 库捕获堆栈信息
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
    
    writeln!(file, "Stack trace:\n{}", backtrace)?;
    println!("堆栈已保存到 stack_trace.txt");
    Ok(())
}

fn main() -> Result<(), Error> {
    // 1. 创建原子变量标记信号
    let term_flag = Arc::new(AtomicBool::new(false));
    
    // 2. 注册需要监听的异常退出信号（SIGTERM、SIGINT、SIGSEGV 等）
    signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&term_flag))?;
    signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&term_flag))?;
    // 注意：SIGSEGV（段错误）等致命信号可能无法被正常捕获，需特殊处理
    
    println!("程序运行中，按 Ctrl+C 或发送 SIGTERM 测试堆栈保存...");
    
    // 3. 主循环：轮询信号状态，正常流程中处理堆栈保存
    while !term_flag.load(Ordering::Relaxed) {
        // 模拟业务逻辑
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    
    // 4. 收到信号后，在正常执行流中保存堆栈
    println!("收到退出信号，准备保存堆栈...");
    save_stack_trace()?;
    
    // 其他清理操作...
    println!("程序已优雅退出");
    Ok(())
}
