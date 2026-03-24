import pyodbc

conn_str = (
    "Driver={ODBC Driver 18 for SQL Server};"
    "Server=myserver.database.windows.net;"
    "Database=mydb;Uid=user;Pwd=***;Encrypt=yes;"
)


def fetch_users():
    with pyodbc.connect(conn_str) as conn:
        cur = conn.cursor()
        cur.execute("SELECT id, name FROM users WHERE active = 1")
        return cur.fetchall()
