"""Azure Virtual Machines — compute.

``ComputeManagementClient`` (mgmt SDK) — not HTTP handlers; no
``functions_framework``.

GCP analogue: ``gcp_reference/compute_engine_manager.py``.
"""
from azure.identity import DefaultAzureCredential
from azure.mgmt.compute import ComputeManagementClient


class VirtualMachineManager:
    """Manages Azure Virtual Machines"""

    def __init__(self, subscription_id, resource_group_name):
        credential = DefaultAzureCredential()
        self.compute_client = ComputeManagementClient(credential, subscription_id)
        self.resource_group_name = resource_group_name

    def create_vm(self, vm_name, location, vm_size, admin_username, admin_password, image_reference):
        """Create a virtual machine"""
        try:
            from azure.mgmt.compute.models import (
                HardwareProfile,
                ImageReference,
                ManagedDiskParameters,
                OSDisk,
                StorageProfile,
                VirtualMachine,
            )

            vm_parameters = VirtualMachine(
                location=location,
                hardware_profile=HardwareProfile(vm_size=vm_size),
                storage_profile=StorageProfile(
                    image_reference=ImageReference(
                        publisher=image_reference['publisher'],
                        offer=image_reference['offer'],
                        sku=image_reference['sku'],
                        version=image_reference['version']
                    ),
                    os_disk=OSDisk(
                        create_option='FromImage',
                        managed_disk=ManagedDiskParameters(storage_account_type='Premium_LRS')
                    )
                ),
                os_profile={
                    'computer_name': vm_name,
                    'admin_username': admin_username,
                    'admin_password': admin_password
                }
            )

            async_vm_creation = self.compute_client.virtual_machines.begin_create_or_update(
                self.resource_group_name,
                vm_name,
                vm_parameters
            )
            vm_result = async_vm_creation.result()
            print(f"Virtual machine {vm_name} created successfully")
            return vm_result
        except Exception as e:
            print(f"Error creating virtual machine: {e}")
            return None

    def list_vms(self):
        """List all virtual machines in resource group"""
        try:
            vms = self.compute_client.virtual_machines.list(self.resource_group_name)
            return list(vms)
        except Exception as e:
            print(f"Error listing virtual machines: {e}")
            return []

    def start_vm(self, vm_name):
        """Start a virtual machine"""
        try:
            async_vm_start = self.compute_client.virtual_machines.begin_start(
                self.resource_group_name,
                vm_name
            )
            async_vm_start.wait()
            print(f"Virtual machine {vm_name} started")
            return True
        except Exception as e:
            print(f"Error starting virtual machine: {e}")
            return False

    def stop_vm(self, vm_name):
        """Stop a virtual machine"""
        try:
            async_vm_stop = self.compute_client.virtual_machines.begin_power_off(
                self.resource_group_name,
                vm_name
            )
            async_vm_stop.wait()
            print(f"Virtual machine {vm_name} stopped")
            return True
        except Exception as e:
            print(f"Error stopping virtual machine: {e}")
            return False

    def delete_vm(self, vm_name):
        """Delete a virtual machine"""
        try:
            async_vm_delete = self.compute_client.virtual_machines.begin_delete(
                self.resource_group_name,
                vm_name
            )
            async_vm_delete.wait()
            print(f"Virtual machine {vm_name} deleted")
            return True
        except Exception as e:
            print(f"Error deleting virtual machine: {e}")
            return False
