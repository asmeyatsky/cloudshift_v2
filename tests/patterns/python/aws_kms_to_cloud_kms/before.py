import boto3

kms = boto3.client('kms')
KEY_ID = 'alias/my-app-key'


def encrypt_data(plaintext: bytes):
    response = kms.encrypt(KeyId=KEY_ID, Plaintext=plaintext)
    return response['CiphertextBlob']


def decrypt_data(ciphertext: bytes):
    response = kms.decrypt(CiphertextBlob=ciphertext)
    return response['Plaintext']
