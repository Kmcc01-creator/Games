# Iced GUI Framework Integration with Macrokid

> Demonstrating how macrokid can simplify Iced GUI development through intelligent proc-macros

## Overview

This example demonstrates how macrokid's proc-macro framework can eliminate boilerplate in Iced GUI applications. Iced is a cross-platform GUI library for Rust inspired by The Elm Architecture, providing type-safe and declarative UI development.

## What is Iced?

Iced is a cross-platform GUI library for Rust that follows The Elm Architecture pattern. It provides:

- **Declarative UI** - Describe what your UI should look like, not how to build it
- **Type Safety** - Compile-time guarantees for UI state and interactions
- **Cross-Platform** - Native performance on Windows, macOS, and Linux
- **Modern Architecture** - Inspired by functional programming patterns from Elm

### Core Architecture Concepts

Iced applications are built around four fundamental concepts:

1. **State/Model** - Your application's data and current state
2. **Messages** - Events that can modify your application state  
3. **Update Function** - Pure function that processes messages and returns new state
4. **View Function** - Pure function that renders UI based on current state

```rust
// Traditional Iced boilerplate
#[derive(Debug, Clone)]
enum Message {
    Increment,
    Decrement,
}

struct Counter {
    value: i32,
}

impl Counter {
    fn update(&mut self, message: Message) {
        match message {
            Message::Increment => self.value += 1,
            Message::Decrement => self.value -= 1,
        }
    }

    fn view(&self) -> Element<Message> {
        column![
            button("+").on_press(Message::Increment),
            text(self.value),
            button("-").on_press(Message::Decrement),
        ].into()
    }
}
```

## Necessary Crates for Iced Development

### Core Dependencies

Note: This proc-macro crate is designed to compile without Iced. The generated code uses
`iced::...` identifiers in your application crate, which must include Iced.

```toml
[dependencies]
iced = "0.12" # In your app crate (not needed to build the proc-macro)
```

### Common Additional Crates

Optional dependencies such as `tokio`, `serde`, images, fonts, and color utilities belong in your
application crate as needed.

### Development Tools

GUI test frameworks and benchmarking crates should be added to your application as needed.

## Development Processes and Use Cases

### 1. Simple Interactive Applications

**Use Case**: Counters, calculators, simple forms

**Process**:
1. Define application state struct
2. Define message enum for all possible interactions
3. Implement update function for state transitions
4. Implement view function for UI rendering
5. Run application with `iced::run()`

**Traditional Boilerplate**: ~50-100 lines for basic counter
**With Macrokid**: ~10-20 lines using derive macros

### 2. Complex Multi-Screen Applications

**Use Case**: Settings panels, multi-tab interfaces, wizards

**Process**:
1. Define screen/page enums
2. Create nested state management
3. Implement routing between screens
4. Handle complex message hierarchies
5. Manage shared state across screens

**Traditional Boilerplate**: ~200-500 lines for multi-screen app
**With Macrokid**: ~50-100 lines using component macros

### 3. Real-Time Data Applications

**Use Case**: Monitoring dashboards, chat applications, live data visualization

**Process**:
1. Integrate with async runtime (tokio)
2. Set up subscriptions for external events
3. Handle streaming data updates
4. Manage WebSocket or network connections
5. Update UI reactively

**Traditional Boilerplate**: ~300-800 lines with networking
**With Macrokid**: ~100-200 lines using subscription macros

### 4. Custom Widget Development

**Use Case**: Specialized controls, data visualization, custom layouts

**Process**:
1. Implement `Widget` trait
2. Handle layout calculations
3. Manage widget state
4. Implement drawing/rendering
5. Handle user input events

**Traditional Boilerplate**: ~200-1000 lines per custom widget
**With Macrokid**: ~50-200 lines using widget derive macros

## Common Development Patterns

### State Management Patterns

```rust
// 1. Simple State (single struct)
struct AppState {
    counter: i32,
    text_input: String,
}

// 2. Hierarchical State (nested components)
struct AppState {
    header: HeaderState,
    sidebar: SidebarState,
    content: ContentState,
}

// 3. Screen-Based State (routing)
enum AppState {
    MainMenu(MainMenuState),
    Settings(SettingsState),
    GameView(GameState),
}
```

### Message Handling Patterns

```rust
// 1. Flat Messages (simple apps)
enum Message {
    ButtonClicked,
    TextChanged(String),
    SliderMoved(f32),
}

// 2. Hierarchical Messages (complex apps)
enum Message {
    Header(HeaderMessage),
    Sidebar(SidebarMessage),
    Content(ContentMessage),
}

// 3. Command Messages (async operations)
enum Message {
    LoadData,
    DataLoaded(Result<Data, Error>),
    SaveData(Data),
}
```

### UI Composition Patterns

```rust
// 1. Builder Pattern (fluent API)
column![
    text("Title").size(24),
    row![
        button("OK").on_press(Message::Ok),
        button("Cancel").on_press(Message::Cancel)
    ].spacing(10)
].padding(20)

// 2. Component Pattern (reusable widgets)
fn create_button(label: &str, message: Message) -> Element<Message> {
    button(text(label).center())
        .padding(10)
        .on_press(message)
        .into()
}

// 3. Conditional Rendering
if self.loading {
    container(text("Loading...")).into()
} else {
    self.render_content()
}
```

## How Macrokid Simplifies Iced Development

### Problem: Repetitive Boilerplate

Traditional Iced development involves significant boilerplate:
- Message enum variants for every interaction
- Match statements in update functions
- Verbose widget composition
- Manual state management

### Solution: Intelligent Proc-Macros

Macrokid can generate this boilerplate automatically:

```rust
// Instead of 50+ lines of boilerplate...
#[derive(IcedApp)]
#[title("Counter App")]
struct CounterApp {
    value: i32,
    
    #[message] // Generates Message::Increment
    fn increment(&mut self) { 
        self.value += 1; 
    }
    
    #[message] // Generates Message::Decrement
    fn decrement(&mut self) { 
        self.value -= 1; 
    }
}

#[iced_view]
impl CounterApp {
    fn view(&self) -> Element<Message> {
        column![
            button!("+", self.increment),
            text!(self.value),
            button!("-", self.decrement)
        ]
    }
}
```

## Next Steps

1. **Basic Counter Example** - Demonstrate fundamental macrokid + iced integration
2. **Multi-Component Example** - Show complex state management simplification  
3. **Custom Widget Example** - Reduce widget development boilerplate
4. **Async Example** - Simplify subscription and command handling

## Resources

- [Official Iced Documentation](https://docs.rs/iced/)
- [Iced Examples Repository](https://github.com/iced-rs/iced/tree/master/examples)
- [Learn Iced Tutorial](https://iced.rs/guide/)
- [Iced Community Discord](https://discord.gg/3xZJ65GAhd)
