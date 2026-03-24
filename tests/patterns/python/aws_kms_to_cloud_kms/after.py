from google.cloud import kms

kms_client = kms.KeyManagementServiceClient()
KEY_NAME = 'projects/my-project/locations/global/keyRings/my-ring/cryptoKeys/my-app-key'


def encrypt_data(plaintext: bytes):
    response = kms_client.encrypt(request={"name": KEY_NAME, "plaintext": plaintext})
    return response.ciphertext


def decrypt_data(ciphertext: bytes):
    response = kms_client.decrypt(request={"name": KEY_NAME, "ciphertext": ciphertext})
    return response.plaintext
