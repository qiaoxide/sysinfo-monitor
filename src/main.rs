use std::{sync::mpsc::{self, Receiver}, thread, time::Duration};
use sysinfo::System;
use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent};
use winit::{
    application::ApplicationHandler,
    event_loop::{ActiveEventLoop, EventLoop},
    window::WindowId,
};

// 定义 App 结构体，所有 UI 相关的组件（TrayIcon）都保留在主线程
struct App {
    quit_id: muda::MenuId,
    menu_channel: muda::MenuEventReceiver,
    tray_channel: tray_icon::TrayIconEventReceiver,
    // TrayIcon 存在主线程中，不跨线程传递
    tray_icon: TrayIcon,
    // 接收后台线程传来的 UI 更新文本
    data_receiver: Receiver<String>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        _event: winit::event::WindowEvent,
    ) {
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // 1. 监听后台线程发来的最新文本，并在主线程中更新菜单栏
        while let Ok(new_title) = self.data_receiver.try_recv() {
            let _ = self.tray_icon.set_title(Some(new_title));
        }

        // 2. 监听退出菜单事件
        if let Ok(event) = self.menu_channel.try_recv() {
            if event.id() == &self.quit_id {
                event_loop.exit();
            }
        }

        // 3. 监听图标点击事件
        if let Ok(_event) = self.tray_channel.try_recv() {
            // 可在此处理点击逻辑
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建事件循环
    let event_loop = EventLoop::new()?;

    // 2. 构建下拉菜单
    let menu = muda::Menu::new();
    let quit_item = muda::MenuItem::new("退出程序", true, None);

    menu.append(&muda::PredefinedMenuItem::about(None, None))?;
    menu.append(&muda::PredefinedMenuItem::separator())?;
    menu.append(&quit_item)?;

    // 3. 在主线程构建 TrayIcon
    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_title("⚡ CPU: 0.0%")
        .build()?;

    // 4. 创建标准库通道，用于子线程向主线程传递文本数据
    let (tx, rx) = mpsc::channel::<String>();

    // 5. 开启后台数据采集线程（只传输 String，String 是 Send 的）
    thread::spawn(move || {
        let mut sys = System::new_all();
        loop {
            sys.refresh_cpu_all();
            let cpu_usage = sys.global_cpu_usage();

            let status_text = format!("⚡ CPU: {:.1}%", cpu_usage);
            
            // 将拼好的文本发给主线程
            if tx.send(status_text).is_err() {
                break; // 主线程退出了，停止采集
            }

            thread::sleep(Duration::from_secs(1));
        }
    });

    // 6. 初始化 App 并启动
    let mut app = App {
        quit_id: quit_item.id().clone(),
        menu_channel: muda::MenuEvent::receiver().clone(),
        tray_channel: TrayIconEvent::receiver().clone(),
        tray_icon, // 留在主线程
        data_receiver: rx,
    };

    event_loop.run_app(&mut app)?;

    Ok(())
}