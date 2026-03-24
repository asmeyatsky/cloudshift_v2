import boto3

ssm = boto3.client('ssm')


def get_param(name: str):
    r = ssm.get_parameter(Name=name, WithDecryption=True)
    return r['Parameter']['Value']


def put_param(name: str, value: str):
    ssm.put_parameter(Name=name, Value=value, Type='SecureString', Overwrite=True)
