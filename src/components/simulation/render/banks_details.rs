use super::*;

impl Simulation {
    pub(in crate::components::simulation::render) fn draw_bank_details(
        &self,
        frame: &mut Frame,
        area: Rect,
        world: &RunningSnapshot,
        tab: &BanksTabSnapshot,
    ) {
        match world.details_page {
            0 => {
                let details = tab
                    .bank_details_lines
                    .iter()
                    .map(|line| Line::from(line.as_str()))
                    .collect::<Vec<_>>();
                frame.render_widget(
                    Paragraph::new(details)
                        .style(theme::primary_text().bg(theme::PANEL_BG))
                        .wrap(Wrap { trim: true })
                        .block(
                            Block::default()
                                .title(" Bank Details (Page 1/4) ")
                                .borders(Borders::ALL)
                                .border_style(theme::neutral_border())
                                .title_style(theme::primary_text())
                                .style(Style::default().bg(theme::PANEL_BG)),
                        ),
                    area,
                );
            }
            1 => {
                let [charts_top, charts_mid, charts_bottom] = Layout::vertical([
                    Constraint::Percentage(34),
                    Constraint::Percentage(33),
                    Constraint::Percentage(33),
                ])
                .areas(area);
                let [reserve_area, deposit_area] =
                    Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .areas(charts_top);
                let [created_area, equity_area] =
                    Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .areas(charts_mid);
                let [run_area, spare_area] =
                    Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .areas(charts_bottom);

                self.draw_bank_history_chart(
                    frame,
                    reserve_area,
                    tab.bank_reserve_history.as_ref(),
                    "Reserve History (Page 2/4)",
                    Color::Cyan,
                    false,
                    false,
                    world.currency_symbol.as_str(),
                    world.selected_country_month,
                    tab.disaster_days.as_ref(),
                );
                self.draw_bank_history_chart(
                    frame,
                    deposit_area,
                    tab.bank_deposit_history.as_ref(),
                    "Deposits History (Page 2/4)",
                    Color::Green,
                    false,
                    false,
                    world.currency_symbol.as_str(),
                    world.selected_country_month,
                    tab.disaster_days.as_ref(),
                );
                self.draw_bank_history_chart(
                    frame,
                    created_area,
                    tab.bank_created_money_history.as_ref(),
                    "Created Money History (Page 2/4)",
                    Color::Yellow,
                    false,
                    false,
                    world.currency_symbol.as_str(),
                    world.selected_country_month,
                    tab.disaster_days.as_ref(),
                );
                self.draw_bank_history_chart(
                    frame,
                    equity_area,
                    tab.bank_equity_history.as_ref(),
                    "Equity History (Page 2/4)",
                    Color::LightMagenta,
                    false,
                    false,
                    world.currency_symbol.as_str(),
                    world.selected_country_month,
                    tab.disaster_days.as_ref(),
                );
                self.draw_bank_history_chart(
                    frame,
                    run_area,
                    tab.bank_run_intensity_history.as_ref(),
                    "Run Intensity History (Page 2/4)",
                    Color::LightRed,
                    true,
                    true,
                    world.currency_symbol.as_str(),
                    world.selected_country_month,
                    tab.disaster_days.as_ref(),
                );
                frame.render_widget(
                    Paragraph::new("")
                        .style(theme::muted_text().bg(theme::PANEL_BG))
                        .block(
                            Block::default()
                                .title(" Bank Charts (Page 2/4) ")
                                .borders(Borders::ALL)
                                .border_style(theme::neutral_border())
                                .style(Style::default().bg(theme::PANEL_BG)),
                        ),
                    spare_area,
                );
            }
            2 => self.draw_event_log_panel(
                frame,
                area,
                world,
                &tab.bank_related_events,
                "Bank Related Events (Page 3/4)",
            ),
            _ => self.draw_bank_depositors_table(frame, area, world, tab),
        }
    }

    fn draw_bank_depositors_table(
        &self,
        frame: &mut Frame,
        area: Rect,
        world: &RunningSnapshot,
        tab: &BanksTabSnapshot,
    ) {
        let total = tab.bank_depositor_rows.len();
        let max_scroll = total.saturating_sub(1);
        let scroll = world.bank_depositors_scroll.min(max_scroll);
        let list_height = usize::from(area.height.saturating_sub(3));
        let visible_rows = tab
            .bank_depositor_rows
            .iter()
            .skip(scroll)
            .take(list_height)
            .enumerate()
            .map(|(idx, row)| {
                let style = if idx.is_multiple_of(2) {
                    Style::default().fg(theme::TEXT_PRIMARY)
                } else {
                    Style::default()
                        .fg(theme::TEXT_PRIMARY)
                        .bg(Color::Rgb(23, 31, 48))
                };
                Row::new([
                    Cell::from(row.entity_kind.clone()),
                    Cell::from(row.entity.clone()),
                    Cell::from(row.balance.clone()),
                    Cell::from(row.share.clone()),
                ])
                .style(style)
            });

        let start = if total == 0 { 0 } else { scroll + 1 };
        let end = if total == 0 {
            0
        } else {
            (scroll + list_height).min(total)
        };
        let header = Row::new([
            Cell::from("Type"),
            Cell::from("Entity"),
            Cell::from(format!("Balance ({})", world.currency_symbol)),
            Cell::from("Share"),
        ])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .bg(Color::Rgb(27, 39, 61))
                .add_modifier(Modifier::BOLD),
        );
        let widths = [
            Constraint::Length(10),
            Constraint::Length(36),
            Constraint::Length(18),
            Constraint::Length(10),
        ];

        frame.render_widget(
            Table::new(visible_rows, widths)
                .header(header)
                .column_spacing(1)
                .block(
                    Block::default()
                        .title(format!(
                            " Bank Depositors (Page 4/4) ({}-{} of {}) ",
                            start, end, total
                        ))
                        .borders(Borders::ALL)
                        .border_style(theme::neutral_border())
                        .title_style(theme::primary_text()),
                ),
            area,
        );
    }
}
