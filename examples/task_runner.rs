//! Task Runner Example - Advanced Level
//!
//! A complete task management system demonstrating:
//! - Complex state management with Vec<Task>
//! - Priority levels (Low, Medium, High)
//! - Custom validation
//! - Advanced statistics
//! - Business logic implementation
//!
//! # Usage
//!
//! ```bash
//! # REPL mode
//! cargo run --example task_runner
//!
//! # CLI mode
//! cargo run --example task_runner -- add "Write documentation" --priority high
//! cargo run --example task_runner -- list
//! cargo run --example task_runner -- complete 1
//! ```

use dynamic_cli::prelude::*;
use dynamic_cli::context::downcast_mut;
use dynamic_cli::utils::{is_blank, parse_int, format_numbered_list};
use std::collections::HashMap;

// ============================================================================
// DOMAIN MODEL
// ============================================================================

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Priority {
    Low,
    Medium,
    High,
}

impl Priority {
    /// Parse priority from string
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "low" | "l" => Ok(Priority::Low),
            "medium" | "med" | "m" => Ok(Priority::Medium),
            "high" | "hi" | "h" => Ok(Priority::High),
            _ => Err(DynamicCliError::Validation(
                dynamic_cli::error::ValidationError::CustomConstraint {
                    arg_name: "priority".to_string(),
                    reason: format!(
                        "Invalid priority '{}'. Must be: low, medium, or high",
                        s
                    ),
                }
            )),
        }
    }

    /// Get priority as display string
    fn as_str(&self) -> &'static str {
        match self {
            Priority::Low => "Low",
            Priority::Medium => "Medium",
            Priority::High => "High",
        }
    }
}

/// A task in the system
#[derive(Debug, Clone)]
struct Task {
    id: usize,
    description: String,
    priority: Priority,
    completed: bool,
}

impl Task {
    /// Create a new task
    fn new(id: usize, description: String, priority: Priority) -> Self {
        Self {
            id,
            description,
            priority,
            completed: false,
        }
    }

    /// Format task for display
    fn format(&self) -> String {
        let status = if self.completed { "✓" } else { " " };
        let priority = match self.priority {
            Priority::High => "🔴",
            Priority::Medium => "🟡",
            Priority::Low => "🟢",
        };
        format!(
            "[{}] {} {} - {}",
            status,
            priority,
            self.description,
            self.priority.as_str()
        )
    }
}

// ============================================================================
// CONTEXT
// ============================================================================

/// Execution context for task runner
#[derive(Default)]
struct TaskRunnerContext {
    tasks: Vec<Task>,
    next_id: usize,
    total_completed: usize,
}

impl TaskRunnerContext {
    /// Add a new task
    fn add_task(&mut self, description: String, priority: Priority) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        
        let task = Task::new(id, description, priority);
        self.tasks.push(task);
        
        id
    }

    /// Find task by ID
    fn find_task_mut(&mut self, id: usize) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id)
    }

    /// Get pending tasks (not completed)
    fn pending_tasks(&self) -> Vec<Task> {
        self.tasks
            .iter()
            .filter(|t| !t.completed)
            .cloned()
            .collect()
    }

    /// Get all tasks
    fn all_tasks(&self) -> Vec<Task> {
        self.tasks.clone()
    }

    /// Complete a task by ID
    fn complete_task(&mut self, id: usize) -> Result<()> {
        let task = self.find_task_mut(id)
            .ok_or_else(|| DynamicCliError::Validation(
                dynamic_cli::error::ValidationError::CustomConstraint {
                    arg_name: "id".to_string(),
                    reason: format!("Task with ID {} not found", id),
                }
            ))?;

        if task.completed {
            return Err(DynamicCliError::Validation(
                dynamic_cli::error::ValidationError::CustomConstraint {
                    arg_name: "id".to_string(),
                    reason: format!("Task {} is already completed", id),
                }
            ));
        }

        task.completed = true;
        self.total_completed += 1;
        
        Ok(())
    }

    /// Delete a task by ID
    fn delete_task(&mut self, id: usize) -> Result<String> {
        let index = self.tasks
            .iter()
            .position(|t| t.id == id)
            .ok_or_else(|| DynamicCliError::Validation(
                dynamic_cli::error::ValidationError::CustomConstraint {
                    arg_name: "id".to_string(),
                    reason: format!("Task with ID {} not found", id),
                }
            ))?;

        let task = self.tasks.remove(index);
        Ok(task.description)
    }

    /// Remove all completed tasks
    fn clear_completed(&mut self) -> usize {
        let initial_len = self.tasks.len();
        self.tasks.retain(|t| !t.completed);
        initial_len - self.tasks.len()
    }

    /// Calculate statistics
    fn calculate_stats(&self) -> TaskStats {
        let total = self.tasks.len();
        let completed = self.tasks.iter().filter(|t| t.completed).count();
        let pending = total - completed;
        
        let high_priority = self.tasks
            .iter()
            .filter(|t| !t.completed && t.priority == Priority::High)
            .count();

        let completion_rate = if total > 0 {
            (completed as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        TaskStats {
            total,
            completed,
            pending,
            high_priority,
            completion_rate,
            total_completed_ever: self.total_completed,
        }
    }
}

impl ExecutionContext for TaskRunnerContext {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Statistics about tasks
struct TaskStats {
    total: usize,
    completed: usize,
    pending: usize,
    high_priority: usize,
    completion_rate: f64,
    total_completed_ever: usize,
}

// ============================================================================
// COMMAND HANDLERS
// ============================================================================

/// Add a new task
struct AddCommand;

impl CommandHandler for AddCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let ctx = downcast_mut::<TaskRunnerContext>(context)
            .ok_or_else(|| DynamicCliError::Execution(
                dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "TaskRunnerContext".to_string(),
                }
            ))?;

        // Get and validate description
        let description = args.get("description")
            .ok_or_else(|| DynamicCliError::Parse(
                dynamic_cli::error::ParseError::MissingArgument {
                    argument: "description".to_string(),
                    command: "add".to_string(),
                }
            ))?;

        if is_blank(description) {
            return Err(DynamicCliError::Validation(
                dynamic_cli::error::ValidationError::CustomConstraint {
                    arg_name: "description".to_string(),
                    reason: "Description cannot be empty".to_string(),
                }
            ));
        }

        // Parse priority
        let priority_str = args.get("priority")
            .map(|s| s.as_str())
            .unwrap_or("medium");
        let priority = Priority::from_str(priority_str)?;

        // Add task
        let id = ctx.add_task(description.clone(), priority);

        println!("✓ Task added with ID {}: {}", id, description);
        println!("  Priority: {}", priority.as_str());

        Ok(())
    }
}

/// List tasks
struct ListCommand;

impl CommandHandler for ListCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let ctx = downcast_mut::<TaskRunnerContext>(context)
            .ok_or_else(|| DynamicCliError::Execution(
                dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "TaskRunnerContext".to_string(),
                }
            ))?;

        // Check if --all flag is present
        let show_all = args.get("all")
            .map(|v| v == "true")
            .unwrap_or(false);

        // FIXED: Clone tasks before filtering to avoid move issues
        let tasks = if show_all {
            ctx.all_tasks()
        } else {
            ctx.pending_tasks()
        };

        if tasks.is_empty() {
            if show_all {
                println!("No tasks yet. Use 'add' to create one.");
            } else {
                println!("No pending tasks! Use 'add' to create one, or 'list --all' to see completed tasks.");
            }
            return Ok(());
        }

        // Display header
        if show_all {
            println!("\nAll Tasks ({}):", tasks.len());
        } else {
            println!("\nPending Tasks ({}):", tasks.len());
        }
        println!("{}", "=".repeat(60));

        // Display tasks
        for task in &tasks {
            println!("  [{}] {}", task.id, task.format());
        }

        println!();

        Ok(())
    }
}

/// Mark a task as completed
struct CompleteCommand;

impl CommandHandler for CompleteCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let ctx = downcast_mut::<TaskRunnerContext>(context)
            .ok_or_else(|| DynamicCliError::Execution(
                dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "TaskRunnerContext".to_string(),
                }
            ))?;

        // Parse task ID
        let id_str = args.get("id")
            .ok_or_else(|| DynamicCliError::Parse(
                dynamic_cli::error::ParseError::MissingArgument {
                    argument: "id".to_string(),
                    command: "complete".to_string(),
                }
            ))?;
        let id = parse_int(id_str, "id")? as usize;

        // FIXED: Get description before calling complete_task (which borrows mutably)
        let description = ctx.tasks
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.description.clone())
            .ok_or_else(|| DynamicCliError::Validation(
                dynamic_cli::error::ValidationError::CustomConstraint {
                    arg_name: "id".to_string(),
                    reason: format!("Task with ID {} not found", id),
                }
            ))?;

        // Complete the task
        ctx.complete_task(id)?;

        println!("✓ Task {} completed: {}", id, description);

        Ok(())
    }
}

/// Delete a task
struct DeleteCommand;

impl CommandHandler for DeleteCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        args: &HashMap<String, String>,
    ) -> Result<()> {
        let ctx = downcast_mut::<TaskRunnerContext>(context)
            .ok_or_else(|| DynamicCliError::Execution(
                dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "TaskRunnerContext".to_string(),
                }
            ))?;

        // Parse task ID
        let id_str = args.get("id")
            .ok_or_else(|| DynamicCliError::Parse(
                dynamic_cli::error::ParseError::MissingArgument {
                    argument: "id".to_string(),
                    command: "delete".to_string(),
                }
            ))?;
        let id = parse_int(id_str, "id")? as usize;

        // Delete task
        let description = ctx.delete_task(id)?;

        println!("✓ Task {} deleted: {}", id, description);

        Ok(())
    }
}

/// Clear all completed tasks
struct ClearCommand;

impl CommandHandler for ClearCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        _args: &HashMap<String, String>,
    ) -> Result<()> {
        let ctx = downcast_mut::<TaskRunnerContext>(context)
            .ok_or_else(|| DynamicCliError::Execution(
                dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "TaskRunnerContext".to_string(),
                }
            ))?;

        let count = ctx.clear_completed();

        if count == 0 {
            println!("No completed tasks to clear.");
        } else {
            println!("✓ Cleared {} completed task(s).", count);
        }

        Ok(())
    }
}

/// Show task statistics
struct StatsCommand;

impl CommandHandler for StatsCommand {
    fn execute(
        &self,
        context: &mut dyn ExecutionContext,
        _args: &HashMap<String, String>,
    ) -> Result<()> {
        let ctx = downcast_mut::<TaskRunnerContext>(context)
            .ok_or_else(|| DynamicCliError::Execution(
                dynamic_cli::error::ExecutionError::ContextDowncastFailed {
                    expected_type: "TaskRunnerContext".to_string(),
                }
            ))?;

        let stats = ctx.calculate_stats();

        println!("\n📊 Task Statistics");
        println!("{}", "=".repeat(40));
        println!("  Total tasks:       {}", stats.total);
        println!("  Completed:         {}", stats.completed);
        println!("  Pending:           {}", stats.pending);
        println!("  High priority:     {}", stats.high_priority);
        println!("  Completion rate:   {:.1}%", stats.completion_rate);
        println!("  Total completed:   {}", stats.total_completed_ever);
        println!();

        Ok(())
    }
}

// ============================================================================
// MAIN
// ============================================================================

fn main() -> Result<()> {
    CliBuilder::new()
        .config_file("examples/configs/task_runner.yaml")
        .context(Box::new(TaskRunnerContext::default()))
        .register_handler("add_handler", Box::new(AddCommand))
        .register_handler("list_handler", Box::new(ListCommand))
        .register_handler("complete_handler", Box::new(CompleteCommand))
        .register_handler("delete_handler", Box::new(DeleteCommand))
        .register_handler("clear_handler", Box::new(ClearCommand))
        .register_handler("stats_handler", Box::new(StatsCommand))
        .build()?
        .run()
}
