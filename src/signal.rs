

use std::iter::Sum;
use sci_rs::signal::filter::{design::*, sosfiltfilt_dyn};
use sci_rs::na::RealField;
use num_traits::Float;


pub fn design_butter_lp<F>(order: usize, lowcut: F, fs: F) -> Vec<Sos<F>>
where
    F: Float + RealField + Sum,
{
    //print!("Building Butterworth filter of order {:?} with lowcut {:?}", order, lowcut);
    // Design Second Order Section (SOS) filter
    let filter = butter_dyn(
        order,
        [lowcut].to_vec(),
        Some(FilterBandType::Lowpass),
        Some(false),
        Some(FilterOutputType::Sos),
        Some(fs),
    );
    let DigitalFilter::Sos(SosFormatFilter {sos}) = filter else {
        panic!("Failed to design filter");
    };
    sos
}

pub fn design_butter_hp<F>(order: usize, highcut: F, fs: F) -> Vec<Sos<F>>
where
    F: Float + RealField + Sum,
{
    //print!("Building Butterworth filter of order {:?} with highcut {:?}", order, highcut);
    // Design Second Order Section (SOS) filter
    let filter = butter_dyn(
        order,
        [highcut].to_vec(),
        Some(FilterBandType::Highpass),
        Some(false),
        Some(FilterOutputType::Sos),
        Some(fs),
    );
    let DigitalFilter::Sos(SosFormatFilter {sos}) = filter else {
        panic!("Failed to design filter");
    };
    sos
}


pub fn design_butter_notch<F>(order: usize, lowcut: F, highcut: F, fs: F) -> Vec<Sos<F>>
where
    F: Float + RealField + Sum,
{
    let filter = butter_dyn(
        order,
        [lowcut, highcut].to_vec(),
        Some(FilterBandType::Bandstop),
        Some(false),
        Some(FilterOutputType::Sos),
        Some(fs),
    );
    let DigitalFilter::Sos(SosFormatFilter {sos}) = filter else {
        panic!("Failed to design filter");
    };
    sos
}

pub fn notch_filter(
    sfreq: f64,
    data: Vec<f64>
) -> Result<Vec<f64>, Box<dyn std::error::Error>> {

    let sos = design_butter_notch(2, 49.0, 51.0, sfreq);
    let data_iter = data.iter();

    let filtered: Vec<f64> = sosfiltfilt_dyn(data_iter, &sos);

    Ok(filtered)
}


pub fn hp_filter(
    lfreq: f64,
    sfreq: f64,
    data: Vec<f64>
) -> Result<Vec<f64>, Box<dyn std::error::Error>> {

    let sos = design_butter_hp(2, lfreq, sfreq);
    let data = data.iter();

    let filtered: Vec<f64> = sosfiltfilt_dyn(data, &sos);


    Ok(filtered)
}

pub fn lp_filter(
    lfreq: f64,
    sfreq: f64,
    data: Vec<f64>
) -> Result<Vec<f64>, Box<dyn std::error::Error>> {

    let sos = design_butter_lp(2, lfreq, sfreq);
    let data_iter = data.iter();

    let filtered: Vec<f64> = sosfiltfilt_dyn(data_iter, &sos);

    Ok(filtered)
}
