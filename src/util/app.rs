use super::rebalance::format_f64;
use super::rebalance::to_f64;
use super::rebalance::{lazy_rebalance, to_vec_display};
use crate::util::rebalance::Asset;
use num::BigRational;
use std::{collections::HashMap, error::Error};
use tui::widgets::{ListState, TableState};

pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn new() -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items: Vec::new(),
        }
    }

    pub fn with_items(items: Vec<T>) -> StatefulList<T> {
        StatefulList {
            state: ListState::default(),
            items,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn unselect(&mut self) {
        self.state.select(None);
    }
}

pub struct StatefulTable {
    pub state: TableState,
    pub items: Vec<Vec<String>>,
}

impl StatefulTable {
    fn new() -> StatefulTable {
        StatefulTable {
            state: TableState::default(),
            items: vec![],
        }
    }
    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

/// Represent the four modes of operation for the GUI
/// Normal just for viewing the portfolio data
/// Editing is for editing the asset values
/// Exec is for entering an amount to rebalance
/// ErrorDisplay is for highlighting that input validation has failed
pub enum InputMode {
    Normal,
    Editing,
    Exec,
    ErrorDisplay,
}

/// This struct holds the current state of the app including tracking three UI input odes
/// and keeping state of the portfolio struct
pub struct App<'a> {
    pub items: StatefulList<(&'a str, usize)>,
    pub table_portfolio: StatefulTable,
    pub table_targets: StatefulTable,
    pub table_results: StatefulTable,
    pub events: Vec<(String, String)>,
    pub input_mode: InputMode,
    /// input entered by the user
    pub input: String,
    pub portfolio: Vec<Asset>,
    /// rebalance amount
    pub contribution_amount: f64,
    /// the error message to display if validation fails
    pub error_msg: String,
    pub path_to_portfolio: String,
}

impl<'a> App<'a> {
    pub fn new() -> App<'a> {
        let mut table_portfolio = StatefulTable::new();
        let mut table_targets = StatefulTable::new();
        let table_results = StatefulTable::new();

        //default path should be moved to an argument or config file
        let path_to_targets = "example/targets.csv";
        let path_to_portfolio = "example/portfolio.csv";
        let _contribution_amount = 10000.00;
        let portfolio_value_index = 1;

        let target_map = create_target_map(path_to_targets);
        let portfolio = create_portfolio(path_to_portfolio, portfolio_value_index, &target_map);

        /*let display_target: Vec<Vec<String>> = target_map
        .into_iter()
        .map(|(symbol, percent)| vec![symbol, format!("{}%", percent.0)])
        .collect();*/

        let display_vecs: Vec<(Vec<String>, Vec<String>)> = portfolio
            .iter()
            .map(|asset| {
                (
                    vec![
                        asset.name.clone(),
                        format!("${}", format_f64(to_f64(&asset.value), 2)),
                    ],
                    vec![
                        asset.name.to_string(),
                        format!(
                            "{}%",
                            format_f64(
                                to_f64(
                                    &(asset.target_allocation_percent.clone()
                                        * BigRational::from_float(100.00).unwrap())
                                ),
                                2
                            )
                        ),
                    ],
                )
            })
            .collect();

        let (display_portfolio, display_target): (Vec<_>, Vec<_>) =
            display_vecs.iter().cloned().unzip();

        table_targets.items = display_target;
        table_portfolio.items = display_portfolio;

        let internal_events = Vec::<(String, String)>::new();
        App {
            items: StatefulList::with_items(vec![]),
            table_portfolio,
            table_targets,
            table_results,
            events: internal_events,
            input_mode: InputMode::Normal,
            input: String::new(),
            portfolio,
            contribution_amount: 0.0,
            error_msg: String::new(),
            path_to_portfolio: path_to_portfolio.to_string(),
        }
    }

    /// Used as a debug log for a widget that is curently not displayed
    pub fn add_custom_event(&mut self, line: String) {
        self.events.push((line, "USER".to_string()));
    }

    /// After the UI is updated this makes sure the underlying portfolio struct matches the asset value
    pub fn update_asset(&mut self, index: usize, new_value: String) {
        let row = &self.table_portfolio.items[index];
        let asset_name = row[0].clone();
        if let Some(asset) = self
            .portfolio
            .iter_mut()
            .find(|asset| asset.name == asset_name)
        {
            let float_parse: f64 = new_value.parse().unwrap();
            asset.value = BigRational::from_float(float_parse).unwrap();
        }
    }

    /// Save the portfolio to the original CSV file after edits are made
    pub fn save_portfolio(&mut self /*path_to_portfolio:&str*/) -> Result<(), Box<dyn Error>> {
        let mut wtr = csv::Writer::from_path(&self.path_to_portfolio)?; //path_to_portfolio)?;

        for asset in &self.portfolio {
            wtr.write_record(&[
                asset.name.to_string(),
                format!("${}", format_f64(to_f64(&asset.value), 2)),
            ])?;
        }

        wtr.flush()?;

        Ok(())
    }

    /// Executes the lazy_rebalance and updates the UI with the results using a helper function
    pub fn lazy_rebalance(&mut self) {
        let balanced_portfolio = lazy_rebalance(self.contribution_amount, &mut self.portfolio);
        //clear out the old results
        self.table_results.items = vec![];
        self.table_results.items = to_vec_display(balanced_portfolio);
    }
}

pub struct Percent(pub f64);

pub fn create_target_map(path_to_targets: &str) -> HashMap<String, Percent> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(path_to_targets)
        .unwrap();

    let mut target_map = HashMap::new();

    for result in reader.records() {
        let record = result.unwrap();

        let asset_name = record.get(0).unwrap().trim().to_string();
        let allocation: Percent = {
            let column = record.get(1).unwrap().trim();

            let allocation = column.parse::<f64>().unwrap();

            if allocation <= 0.0 {
                continue;
            }

            Percent(allocation)
        };

        target_map.insert(asset_name, allocation);
    }

    target_map
}

pub fn create_portfolio(
    path_to_portfolio: &str,
    portfolio_value_index: usize,
    target_map: &HashMap<String, Percent>,
) -> Vec<Asset> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(path_to_portfolio)
        .unwrap();

    let mut portfolio_map: HashMap<String, Asset> = HashMap::new();

    for result in reader.records() {
        let record = result.unwrap();

        let asset_name = record.get(0).unwrap().trim().to_string();

        let value = {
            let value: String = record
                .get(portfolio_value_index)
                .unwrap()
                .trim()
                .chars()
                .skip(1)
                .collect();

            value.parse::<f64>().unwrap()
        };

        match target_map.get(&asset_name) {
            None => {}
            Some(&Percent(target_allocation_percent)) => {
                let target_allocation_percent =
                    adjust_target_allocation_percent(target_allocation_percent);

                let asset = Asset::new(asset_name.clone(), target_allocation_percent, value);

                portfolio_map.insert(asset_name, asset);
            }
        }
    }

    for asset_name in target_map.keys() {
        if portfolio_map.contains_key(asset_name) {
            continue;
        }

        let &Percent(target_allocation_percent) = target_map.get(asset_name).unwrap();

        let target_allocation_percent = adjust_target_allocation_percent(target_allocation_percent);

        let asset = Asset::new(asset_name.clone(), target_allocation_percent, 0.0);

        portfolio_map.insert(asset_name.to_string(), asset);
    }

    let mut portfolio = vec![];

    for (_asset_name, asset) in portfolio_map {
        portfolio.push(asset);
    }

    portfolio
}

pub fn adjust_target_allocation_percent(target_allocation_percent: f64) -> f64 {
    target_allocation_percent / 100.0
}
