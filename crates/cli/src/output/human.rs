

use prism_core::types::report::DiagnosticReport;

use crate::output::renderers::{
    render_section_header, render_error_card, render_fix_list, BudgetBar, render_fee_breakdown,
};

pub fn print_report(report: &DiagnosticReport) -> anyhow::Result<()> {
    println!("{}", render_error_card(report));
    println!();

    println!("{}", render_section_header("Transaction Summary"));
    println!(
        "Error: {} ({}:{})",
        report.error_name, report.error_category, report.error_code
    );
    println!("Summary: {}", report.summary);

    if let Some(context) = &report.transaction_context {
        println!();
        println!("{}", render_section_header("Resource Usage"));
        println!(
            "{}",
            BudgetBar::new(
                "CPU",
                context.resources.cpu_instructions_used,
                context.resources.cpu_instructions_limit
            )
            .render()
        );
        println!(
            "{}",
            BudgetBar::new(
                "RAM",
                context.resources.memory_bytes_used,
                context.resources.memory_bytes_limit
            )
            .render()
        );
        println!();
        print!("{}", render_fee_breakdown(&context.fee));
    }

    if !report.suggested_fixes.is_empty() {
        println!();
        println!("{}", render_fix_list(&report.suggested_fixes));
    }

    Ok(())
}
