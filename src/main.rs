use chrono::{Duration, NaiveDate, Utc};
use eval::eval;
use slint::{Model, SharedString, VecModel};
use std::{ collections::BTreeMap, ops::Range, rc::Rc, thread};
use anyhow::{Result, Error};
use rand::Rng;

extern crate simple_excel_writer as excel;

use excel::*;

extern crate eval;

slint::include_modules!();

const DATE_FORMATTER: &str =  "%Y-%m-%d";

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;

    let start_date = SharedString::from(Utc::now().format(DATE_FORMATTER).to_string());
    let end_date = SharedString::from((Utc::now() + Duration::days(1)).format(DATE_FORMATTER).to_string());
    ui.set_start_date(start_date);
    ui.set_end_date(end_date);

    let serials = Rc::new(VecModel::from(vec![
        Serial {min: 0, max: 20},
        Serial {min: 0, max: 20},
        Serial {min: 0, max: 20},
    ]));

    ui.set_serials(serials.clone().into());

    {
        let ui_handler = ui.as_weak();
        ui.on_generate(move || {
            let ui = ui_handler.unwrap();
            
            ui.set_btn_enabled(false);

            let start_date = ui.get_start_date();
            let end_date = ui.get_end_date();

            let test = ui.get_carry_max_1();
            println!("{}, {}, {}", start_date, end_date, test);

            let mut map = BTreeMap::<String, Vec<String>>::new();

            let start = NaiveDate::parse_from_str(&start_date, DATE_FORMATTER).unwrap();
            let end = NaiveDate::parse_from_str(&end_date, DATE_FORMATTER).unwrap();
            let duration = end - start;
            let days = duration.num_days();
            for i in 0..days {
                let date = start + Duration::days(i as i64);
                let day = date.format(DATE_FORMATTER).to_string();
                ui.set_notify(format!("{} 开始生成", day).into());
                
                common(&ui, &mut map, &day).expect("普通计算生成出错");
                ui.set_notify(format!("{} 完成普通计算", day).into());
                carry(&ui, &mut map, &day).expect("进位加生成出错");
                ui.set_notify(format!("{} 完成进位加", day).into());
                serial(&ui, &mut map, &day).expect("连加连减生成出错");
                ui.set_notify(format!("{} 完成连加连减", day).into());
            }
            ui.set_notify(format!("{}-{} 算式已生成完毕", start_date, end_date).into());

            ui.set_notify(format!("{}-{} 开始写入excel", start_date, end_date).into());
            let file_name = Utc::now().format("%Y%m%d%H%M%S").to_string();
            let mut wb = Workbook::create(format!("D:/口算{}_{}_{}.xlsx", start_date, end_date, file_name).as_str());
            let mut sheet = wb.create_sheet("SheetName");
            
            sheet.add_column(Column{width: 18.0});
            sheet.add_column(Column{width: 18.0});
            sheet.add_column(Column{width: 18.0});
            sheet.add_column(Column{width: 18.0});
            sheet.add_column(Column{width: 18.0});

            wb.write_sheet(&mut sheet, |sheet_writer| {
                let sw = sheet_writer;
                for day in map.keys().into_iter() {
                    /* sheet.merged_cells.push(MergedCell {
                        start_ref: ref_id(0, line),
                        end_ref: ref_id(4, line),
                    }); */
                    sw.append_row(row![day.as_ref()])?;
                    let arr = map.get(day).unwrap();
                    
                    for i in (0..arr.len()).step_by(5) {
                        let mut row = Row::new();
                        for j in 0..5 {
                            if i+j > arr.len() - 1 {
                                break;
                            }
                            row.add_cell(format!("{}{}", arr[i+j], "="));
                        }
                        sw.append_row(row)?;
                    }
                }
                sw.append_row(row![])
            }).expect("写入excel出错");

            wb.close().expect("close excel error!");

            ui.set_notify(format!("{}-{} 写入excel完毕，任务完成", start_date, end_date).into());
            ui.set_btn_enabled(true);
        })
    }

    {
        let serials = serials.clone();
        ui.on_serial_num_changed(move |num| {
            let cnt: i32 = serials.row_count().try_into().unwrap();
            if num > cnt {
                let n = num - cnt;
                for _ in 0..n {
                    serials.push(Serial{min: 0, max: 20});
                }
            }else if num < cnt {
                let n = cnt - num;
                for _ in 0..n {
                    serials.remove(serials.row_count() - 1);
                }
            }
        })
    }

    ui.run()
}

fn common(ui: &AppWindow, 
    map: &mut BTreeMap<String, Vec<String>>, 
    day: &str) -> Result<(), Error> {
    let total = ui.get_common_total();
    let min = ui.get_common_min();
    let max = ui.get_common_max();

    if !map.contains_key(day) {
        map.insert(day.to_string(), vec![]);
    }

    for _ in 0..total {
        loop {
            let mut first = gen_rand(min, max);
            let mut second = gen_rand(min, max);
            let op = gen_op();

            if op == "-" && first < second {
                let tmp = first;
                first = second;
                second = tmp;
            }

            let equation = format!("{}{}{}", first, op, second);
            if !map[day].contains(&equation) {
                 println!("{}", equation);
                map.get_mut(day).unwrap().push(equation);
                break;
            }
            thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    Ok(())
}

fn carry(ui: &AppWindow, 
    map: &mut BTreeMap<String, Vec<String>>, 
    day: &str) -> Result<(), Error> {
        let total = ui.get_carry_total();
        let min_1 = ui.get_carry_min_1();
        let max_1 = ui.get_carry_max_1();
        let min_2 = ui.get_carry_min_2();
        let max_2 = ui.get_carry_max_2();

        if !map.contains_key(day) {
            map.insert(day.to_string(), vec![]);
        }

        for _ in 0..total {
            loop {
                let mut first = gen_rand(min_1, max_1);
                let mut second = gen_rand(min_2, max_2);
                let op = "+";

                if op == "-" && first < second {
                    let tmp = first;
                    first = second;
                    second = tmp;
                }

                if first%10 + second%10 >= 10 {
                    let equation = format!("{}{}{}", first, op, second);
                    if !map[day].contains(&equation) {
                        println!("{}", equation);
                        map.get_mut(day).unwrap().push(equation);
                        break;
                    }
                }

                thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        Ok(())
}

fn serial(ui: &AppWindow, 
    map: &mut BTreeMap<String, Vec<String>>, 
    day: &str) -> Result<(), Error> {
        if !map.contains_key(day) {
            map.insert(day.to_string(), vec![]);
        }
                
        let total = ui.get_serial_total();
        let limit = ui.get_serial_limit();
        let serials = ui.get_serials();

        for _ in 0..total {
            let mut expr = String::from("");
            let mut j = 0;
            for serial in serials.iter() {
                let min = serial.min;
                let max = serial.max;
                
                loop {
                    let n = gen_rand(min, max);
                    if j == 0 {
                        expr.push_str(format!("{}",n).as_str());
                        j = j + 1;
                        break;
                    }
                    let op = gen_op();
                    let mut expr_tmp = expr.clone();
                    expr_tmp.push_str(op.as_str());
                    expr_tmp.push_str(format!("{}",n).as_str());
                    let result = eval(&expr_tmp).unwrap().as_i64().unwrap() as i32;

                    if result >= 0 && result <= limit {
                        expr = expr_tmp;
                        break;
                    }

                    thread::sleep(std::time::Duration::from_millis(10));
                }
            }

            if !map[day].contains(&expr) {
                println!("{}", expr);
                map.get_mut(day).unwrap().push(expr);
            }
        }

        Ok(())
}

fn gen_rand(start: i32, end: i32) -> i32 {
    let mut rng = rand::thread_rng();
    rng.gen_range(Range{start, end})
}

fn gen_op() -> String {
    let n = gen_rand(0, 100);
    if n % 2 == 0 {
        "+".to_string()
    } else {
        "-".to_string()
    }
}
