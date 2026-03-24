import boto3

rds = boto3.client('rds-data')
RESOURCE_ARN = 'arn:aws:rds:us-east-1:123:cluster:mydb'
SECRET_ARN = 'arn:aws:secretsmanager:us-east-1:123:secret:rds'


def run_query(sql: str):
    return rds.execute_statement(
        resourceArn=RESOURCE_ARN,
        secretArn=SECRET_ARN,
        database='app',
        sql=sql,
    )
