use chrono::{Duration, NaiveDate, NaiveTime, Utc};
use sqlx::{
    postgres::PgPool,
    postgres::{PgPoolOptions, PgQueryResult},
    Pool, Postgres, Row,
};
// use log::{error, info};
// use sqlx::{postgres::PgRow, FromRow, QueryBuilder, Row};

use tools::time_determine;
#[macro_use]
extern crate dotenv_codegen;

pub mod tools;
pub type DBResult<T> = Result<T, sqlx::Error>;

#[derive(Debug, Clone)]
pub enum ArgDay {
    Today,
    Tommorow,
    Aftertommorow,
}

impl ArgDay {
    pub fn get_date(arg: ArgDay) -> NaiveDate {
        use ArgDay::*;

        match arg {
            Today => (Utc::now() + Duration::hours(3)).date_naive(),
            Tommorow => (Utc::now() + Duration::hours(3) + Duration::days(1)).date_naive(),
            Aftertommorow => (Utc::now() + Duration::hours(3) + Duration::days(2)).date_naive(),
        }
    }
}

#[derive(Debug, PartialEq, Default)]
pub struct Movie {
    pub title: String,
    pub year: Option<i32>,
    pub genre: Option<String>,
    pub director: Option<String>,
    pub description: Option<String>,
    pub href_moskino: Option<String>,
    pub href_kinopoisk: Option<String>,
    pub country: Option<String>,
    pub duration: Option<i32>,
    pub age: Option<i32>,
    pub tagline: Option<String>,
}

#[derive(Debug)]
pub struct MovieShort {
    pub id: i32,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct Cinema {
    pub id: i32,
    pub name: String,
}

impl Cinema {
    pub fn new(id: i32, name: String) -> Self {
        Self { id, name }
    }
}

#[derive(Debug)]
pub struct Session {
    pub cinema_name: String,
    pub showtime: NaiveTime,
    pub showdate: NaiveDate,
    pub price: i32,
}

pub struct DB {
    pub conn: Pool<Postgres>,
}

impl DB {
    pub async fn pool() -> DBResult<Self> {
        let conn = PgPoolOptions::new()
            .max_connections(dotenv!("DATABASE_MAX_CONNECTIONS").parse().unwrap())
            .connect(dotenv!("DATABASE_URL"))
            .await?;

        Ok(Self { conn })
    }

    pub async fn new() -> DBResult<Self> {
        Ok(Self {
            conn: PgPool::connect(dotenv!("DATABASE_URL")).await?,
        })
    }

    pub async fn q_get_sessions_by_cinema(conn: impl sqlx::PgExecutor<'_>, movie_id: i32, cinema_id: i32, date: NaiveDate) -> DBResult<Option<Vec<Session>>> {
        let time = time_determine(date);

        let sessions: Vec<Session> = sqlx::query_as!(
            Session,
            r#"
            SELECT
                c.name as cinema_name,
                s.showtime as showtime,
                s.showdate as showdate,
                s.price as price
            FROM
                moskino.sessions s
            JOIN
                moskino.movies m ON s.movie_id = m.movie_id
            JOIN
                moskino.cinemas c ON s.cinema_id = c.cinema_id
            WHERE
            	s.movie_id = $1
            AND
                s.cinema_id = $2
            AND
            	s.showdate = $3
            AND
                s.showtime >= $4
            ORDER BY
                cinema_name, showtime;
            "#,
            movie_id,
            cinema_id,
            date,
            time
        )
        .fetch_all(conn)
        .await?;

        if sessions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(sessions))
        }
    }

    // выборка сеансов по фильму по всем кинотеатрам за определенную дату
    pub async fn q_get_sessions_all(conn: impl sqlx::PgExecutor<'_>, movie_id: i32, date: NaiveDate) -> DBResult<Option<Vec<Session>>> {
        let time = time_determine(date);

        let sessions: Vec<Session> = sqlx::query_as!(
            Session,
            r#"
            SELECT
                c.name as cinema_name,
                s.showtime as showtime,
                s.showdate as showdate,
                s.price as price
            FROM
                moskino.sessions s
            JOIN
                moskino.movies m ON s.movie_id = m.movie_id
            JOIN
                moskino.cinemas c ON s.cinema_id = c.cinema_id
            WHERE
            	s.movie_id = $1
            AND
            	s.showdate = $2
            AND
                s.showtime >= $3
            ORDER BY
                cinema_name, showtime;
            "#,
            movie_id,
            date,
            time
        )
        .fetch_all(conn)
        .await?;

        if sessions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(sessions))
        }
    }

    pub async fn q_get_cinemas(conn: impl sqlx::PgExecutor<'_>) -> DBResult<Option<Vec<Cinema>>> {
        let cinemas: Vec<Cinema> = sqlx::query_as!(
            Cinema,
            r#"
            SELECT
                cinema_id as id, name
            FROM
                moskino.cinemas
            WHERE
                is_active = true;
            "#,
        )
        .fetch_all(conn)
        .await?;

        if cinemas.is_empty() {
            Ok(None)
        } else {
            Ok(Some(cinemas))
        }
    }

    pub async fn q_get_cinema_name_by_id(conn: impl sqlx::PgExecutor<'_>, cinema_id: i32) -> DBResult<Option<String>> {
        sqlx::query_scalar!(
            r#"
            SELECT
                name
            FROM
                moskino.cinemas
            WHERE
                cinema_id = $1
            AND
                is_active = true
            ;"#,
            cinema_id
        )
        .fetch_optional(conn)
        .await
    }

    // TODO
    pub async fn q_count_movies_by_cinema(conn: impl sqlx::PgExecutor<'_>, date: NaiveDate, cinema_id: i32) -> DBResult<i64> {
        let time = time_determine(date);

        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(DISTINCT m.movie_id)
            FROM
                moskino.cinemas c
            JOIN
                moskino.sessions s ON c.cinema_id = s.cinema_id
            JOIN
                moskino.movies m ON s.movie_id = m.movie_id
            WHERE
                c.cinema_id = $1
            AND
                s.showdate = $2
            AND
                s.showtime >= $3
            ;"#,
        )
        .bind(cinema_id)
        .bind(date)
        .bind(time)
        .fetch_one(conn)
        .await
    }

    pub async fn q_get_movie_by_id(conn: impl sqlx::PgExecutor<'_>, movie_id: i32) -> DBResult<Movie> {
        let row = sqlx::query(
            r#"
            SELECT
                title,
                year,
                genre,
                director,
                description,
                href_moskino,
                href_kinopoisk
            FROM
                moskino.movies
            WHERE
                movie_id = $1;
            "#,
        )
        .bind(movie_id)
        .fetch_one(conn)
        .await?;

        Ok(Movie {
            title: row.get("title"),
            year: row.get("year"),
            genre: row.get("genre"),
            director: row.get("director"),
            description: row.get("description"),
            href_moskino: row.get("href_moskino"),
            href_kinopoisk: row.get("href_kinopoisk"),
            ..Default::default()
        })
    }

    // TODO
    pub async fn q_get_movies_short(conn: impl sqlx::PgExecutor<'_>, date: NaiveDate, page: i64, items_per_page: i64) -> DBResult<Option<Vec<MovieShort>>> {
        let time = time_determine(date);
        let offset = (page - 1) * items_per_page;

        let movies = sqlx::query_as!(
            MovieShort,
            r#"
            SELECT DISTINCT
                m.movie_id as id,
                m.title
            FROM
                moskino.movies m
            JOIN
                moskino.sessions s ON m.movie_id = s.movie_id
            WHERE
                s.showdate = $1
            AND
                s.showtime >= $2
            LIMIT
                $3
            OFFSET
                $4
            ;"#,
            date,
            time,
            items_per_page,
            offset
        )
        .fetch_all(conn)
        .await?;

        if movies.is_empty() {
            Ok(None)
        } else {
            Ok(Some(movies))
        }
    }

    pub async fn q_get_movies_short_by_cinema(
        conn: impl sqlx::PgExecutor<'_>,
        date: NaiveDate,
        cinema_id: i32,
        page: i64,
        items_per_page: i64,
    ) -> DBResult<Option<Vec<MovieShort>>> {
        let time = time_determine(date);
        let offset = (page - 1) * items_per_page;

        let movies = sqlx::query_as!(
            MovieShort,
            r#"
            SELECT DISTINCT
                m.movie_id as id,
                m.title
            FROM
                moskino.cinemas c
            JOIN
                moskino.sessions s ON c.cinema_id = s.cinema_id
            JOIN
                moskino.movies m ON s.movie_id = m.movie_id
            WHERE
                c.cinema_id = $1
            AND
                s.showdate = $2
            AND
                s.showtime >= $3
            --ORDER BY
            --    m.title
            LIMIT
                $4
            OFFSET
                $5
            "#,
            cinema_id,
            date,
            time,
            items_per_page,
            offset
        )
        .fetch_all(conn)
        .await?;

        if movies.is_empty() {
            Ok(None)
        } else {
            Ok(Some(movies))
        }
    }

    pub async fn q_count_movies(conn: impl sqlx::PgExecutor<'_>, date: NaiveDate) -> DBResult<i64> {
        let time = time_determine(date);

        sqlx::query_scalar::<_, i64>(
            r#"
                SELECT COUNT(DISTINCT m.movie_id)
                FROM
                    moskino.movies m
                JOIN
                    moskino.sessions s ON m.movie_id = s.movie_id
                WHERE
                    s.showdate = $1
                AND
                    s.showtime >= $2
            ;"#,
        )
        .bind(date)
        .bind(time)
        .fetch_one(conn)
        .await
    }

    pub async fn insert_session(&self, session: &Session, cinema_id: i32, movie_id: i32) -> DBResult<PgQueryResult> {
        sqlx::query!(
            r#"
                INSERT INTO
                    moskino.sessions (cinema_id, movie_id, showdate, showtime, price)
                VALUES
                    ($1, $2, $3, $4, $5);
                "#,
            cinema_id,
            movie_id,
            session.showdate,
            session.showtime,
            session.price
        )
        .execute(&self.conn)
        .await
    }

    // insert cinema into moskino.cinema
    // returns id of inserted cinema (or already existed)
    pub async fn insert_cinema(&self, cinema: &Cinema) -> DBResult<i32> {
        sqlx::query_scalar!(
            r#"
            INSERT INTO
                moskino.cinemas (name, is_active)
            VALUES
                ($1, true)
            ON CONFLICT (name) DO UPDATE
            SET
                name = excluded.name
            RETURNING
                cinema_id;
            "#,
            cinema.name
        )
        .fetch_one(&self.conn)
        .await
    }

    // insert movie into moskino.movie
    // returns id of inserted movie (or already existed)
    pub async fn insert_movie(&self, movie: &Movie) -> DBResult<i32> {
        sqlx::query_scalar!(
            r#"
            INSERT INTO
                moskino.movies (title, year, genre, country, duration, age, director, tagline, description, href_moskino, href_kinopoisk)
            VALUES
                ($1, COALESCE($2, 0), $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (title, year) DO UPDATE
            SET
                title = excluded.title
            RETURNING
                movie_id;
            "#,
            movie.title,
            movie.year,
            movie.genre,
            movie.country,
            movie.duration,
            movie.age,
            movie.director,
            movie.tagline,
            movie.description,
            movie.href_moskino,
            movie.href_kinopoisk
        )
        .fetch_one(&self.conn)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_q_get_movie_by_id() {
        let db = DB::new().await.unwrap();

        let movie_test = Movie {
            title: "Test Movie".to_string(),
            year: Some(2023),
            genre: Some("Action".to_string()),
            director: Some("Test Director".to_string()),
            description: Some("This is a description of the test movie.".to_string()),
            href_moskino: Some("http://example.com/moskino/test-movie".to_string()),
            href_kinopoisk: Some("http://example.com/kinopoisk/test-movie".to_string()),
            ..Default::default()
        };

        let movie = DB::q_get_movie_by_id(&db.conn, 1).await.unwrap();

        assert_eq!(movie, movie_test);
    }
}
