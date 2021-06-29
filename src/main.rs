/*!
 * This is to control the **DeskPi Pro Fan** using CPU's temperature
 */

use std::collections::HashSet;
use std::env;

use systemstat::{Platform, System};

/** A point in the fan curve */
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct Point {
    temperature: u8,
    speed: u8,
}

impl Point {
    pub fn new(temperature: u8, speed: u8) -> Self {
        Point { temperature, speed }
    }
}

impl std::str::FromStr for Point {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        /* Split the string on `:` and match for two parts */
        let (temperature, speed) = match s
            .splitn(2, ':')
            .map(|num| num.parse().map_err(|_| ()))
            .collect::<Vec<Result<u8, ()>>>()[..]
        {
            [left, right] => (left?, right?),
            _ => return Err(()),
        };

        Ok(Point { temperature, speed })
    }
}

/** A curve containing points inside it */
#[derive(Debug)]
struct Curve(Vec<Point>);

impl Curve {
    pub fn from_points(points: Vec<Point>) -> Self {
        Curve(points)
    }

    fn calculate(&self, temperature: f32) -> u8 {
        let (lower, upper) = self.bounds(temperature);

        /* Calculate the difference in speed and temperature between the lower and upper bounds */
        let (tempdiff, speeddiff) = (
            upper.temperature - lower.temperature,
            upper.speed - lower.speed,
        );
        /* Calculate the percent of temperature overflow from the lower bound relative to the upper
         * bound */
        let percent = (temperature - lower.temperature as f32) / tempdiff as f32;

        /* Calculate speed from the proportionnal percentage of temperature */
        (lower.speed as f32 + (speeddiff as f32 * percent)) as u8
    }

    fn bounds(&self, temperature: f32) -> (Point, Point) {
        let mut iter = self.0.clone().into_iter().peekable();

        loop {
            /* Get lower and upper bounds for the current temperature */
            match (iter.next(), iter.peek()) {
                (Some(point), None) => break (point.clone(), point),
                (Some(point), Some(next))
                    if temperature > (point.temperature as f32)
                        && temperature < (next.temperature as f32) =>
                {
                    break (point, next.clone())
                }
                (Some(_), Some(_)) => continue,
                _ => panic!(),
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sys = System::new();

    /* Get points from the command line */
    let points: HashSet<Point> = std::iter::once(Ok(Point::new(0, 0)))
        .chain(env::args().skip(1).map(|s| s.parse()))
        .collect::<Result<_, _>>()
        .map_err(|_| "Malformed input argument, the correct format is <temperature>:<speed>.")?;

    /* Push them into a vector and sort it */
    let mut points: Vec<_> = points.into_iter().collect();
    points.sort();

    if points.len() < 2 {
        return Err("You must provide at least one more point in order to make a fan curve.".into())
    }

    /* Obtain a curve from those points and the temperature from the sensors */
    let curve = Curve::from_points(points);
    let temperature = sys.cpu_temp()?;

    let pwm = curve.calculate(temperature);

    eprintln!(
        ":i: Current temperature of `{}`, computed fan speed of `{}`",
        temperature, pwm
    );
    println!("pwm_{:03}", pwm);

    Ok(())
}
