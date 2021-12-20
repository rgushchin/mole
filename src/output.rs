use std::cmp::Ordering;
pub enum Data {
    UInt(u64),
    Int(i64),
    Float(f64),
    Text(String),
}

pub struct Column {
    pub title: String,
    pub width: usize,
}

pub struct Table {
    pub columns: Vec<Column>,
    pub data: Vec<Vec<Data>>,
    pub sort_by: Option<usize>,
    pub filter_by: Option<usize>,
}

fn default_fmt(data: &Data, column: &Column) -> String {
    match data {
        Data::UInt(v) => format!("{1:<0$}", column.width, v),
        Data::Int(v) => format!("{1:<0$}", column.width, v),
        Data::Float(v) => format!("{1:<0$.1}", column.width, v),
        Data::Text(v) => format!("{1:<0$}", column.width, v),
    }
}

fn compare_data(a: &Data, b: &Data) -> Ordering {
    match (a, b) {
        (Data::UInt(x), Data::UInt(y)) => x.cmp(&y),
        (Data::Int(x), Data::Int(y)) => x.cmp(&y),
        (Data::Float(x), Data::Float(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Less),
        (Data::Text(x), Data::Text(y)) => x.cmp(&y),
        (_, _) => panic!(),
    }
}

impl Table {
    pub fn add_row(&mut self, data: Vec<Data>) {
        assert_eq!(self.columns.len(), data.len());
        self.data.push(data);
    }

    pub fn display_table(&mut self) -> String {
        let mut output = String::new();
        let delimiter = String::from(" ");
        let newline = String::from("\n");

        // sort data
        if self.sort_by.is_some() {
            let sort_by = self.sort_by.unwrap();
            self.data
                .sort_unstable_by(|a, b| compare_data(&a[sort_by], &b[sort_by]));
        }

        // print titles
        for column in &self.columns {
            output.push_str(&format!("{:width$}", column.title, width = column.width));
            output.push_str(&delimiter);
        }
        output.push_str(&newline);
        output.push_str(&"-".repeat(output.len() - 2));
        output.push_str(&newline);

        // print data
        for row in &self.data {
            let mut i = 0;
            for cell in row {
                output.push_str(&default_fmt(cell, &self.columns[i]));
                output.push_str(&delimiter);
                i += 1;
            }
            output.push_str(&newline);
        }

        output
    }

    pub fn clear_data(&mut self) {
        self.data.clear();
    }
}

#[macro_export]
macro_rules! table {
    ( $( $x:expr ),* ) => {
	$crate::output::Table {
	    columns: vec![$($crate::output::Column {title: $x.0.to_string(), width: $x.1}),*],
	    data: vec![],
	    sort_by: Some(0),
	    filter_by: None,
	}
    }
}

#[test]
fn create_test_table() {
    let mut t = table![("Pid", 8), ("Comm", 16), ("usr%", 4), ("Sys%", 4)];

    t.add_row(vec![
        Data::Int(1),
        Data::Text("aaa".to_string()),
        Data::Float(0.1),
        Data::Float(0.01),
    ]);
    t.add_row(vec![
        Data::Int(2),
        Data::Text("bbb".to_string()),
        Data::Float(0.2),
        Data::Float(0.0),
    ]);
    t.add_row(vec![
        Data::Int(3),
        Data::Text("ccc".to_string()),
        Data::Float(0.0),
        Data::Float(0.04),
    ]);
    t.add_row(vec![
        Data::Int(4),
        Data::Text("ddd".to_string()),
        Data::Float(0.0),
        Data::Float(0.05),
    ]);

    println!("{}", t.display_table());
}
