import zmq
import struct
import uuid
import logging
import os
import requests
import json
from urllib.parse import urljoin
from datafed import SDMS_Auth_pb2 as auth
from datafed import SDMS_pb2 as sdms
from datafed import SDMS_Anon_pb2 as anon

def load_key_file(key_path):
    """Load key from file and return as string"""
    try:
        with open(key_path, 'r') as f:
            key = f.read().strip()
            print(f"Loaded key from {key_path}: {key[:10]}...")
            return key
    except Exception as e:
        print(f"Error loading key from {key_path}: {e}")
        return None

class RestApiClient:
    """
    RESTful API client for DataFed web service.
    Tests the HTTP REST endpoints of the DataFed web service.
    """
    
    def __init__(self, base_url="http://localhost:8080", timeout=30):
        """
        Initialize REST API client.
        
        Args:
            base_url: Base URL of the DataFed web service
            timeout: Request timeout in seconds
        """
        self.base_url = base_url.rstrip('/')
        self.timeout = timeout
        self.session = requests.Session()
        self.session.headers.update({
            'Content-Type': 'application/json',
            'Accept': 'application/json'
        })
        
    def _make_request(self, method, endpoint, **kwargs):
        """Make HTTP request with error handling"""
        url = urljoin(self.base_url, endpoint)
        try:
            response = self.session.request(method, url, timeout=self.timeout, **kwargs)
            response.raise_for_status()
            return response
        except requests.exceptions.RequestException as e:
            print(f"✗ HTTP {method} {endpoint}: {e}")
            return None
    
    def test_health_check(self):
        """Test basic connectivity"""
        print("Testing web service connectivity...")
        response = self._make_request('GET', '/')
        if response:
            print("✓ Web service is accessible")
            return True
        else:
            print("✗ Web service is not accessible")
            return False
    
    def test_user_endpoints(self):
        """Test user management endpoints"""
        print("\n=== Testing User Management Endpoints ===")
        
        # Test user list
        print("1. Testing user list...")
        response = self._make_request('GET', '/api/usr/list/all')
        if response:
            print("✓ User list endpoint accessible")
        else:
            print("✗ User list endpoint failed")
        
        # Test user search
        print("2. Testing user search...")
        response = self._make_request('GET', '/api/usr/find/by_name_uid', 
                                    params={'name_uid': 'test'})
        if response:
            print("✓ User search endpoint accessible")
        else:
            print("✗ User search endpoint failed")
    
    def test_project_endpoints(self):
        """Test project management endpoints"""
        print("\n=== Testing Project Management Endpoints ===")
        
        # Test project list
        print("1. Testing project list...")
        response = self._make_request('GET', '/api/prj/list')
        if response:
            print("✓ Project list endpoint accessible")
        else:
            print("✗ Project list endpoint failed")
        
        # Test project search
        print("2. Testing project search...")
        search_data = {
            "query": "test",
            "offset": 0,
            "count": 10
        }
        response = self._make_request('POST', '/api/prj/search', 
                                    json=search_data)
        if response:
            print("✓ Project search endpoint accessible")
        else:
            print("✗ Project search endpoint failed")
    
    def test_data_endpoints(self):
        """Test data management endpoints"""
        print("\n=== Testing Data Management Endpoints ===")
        
        # Test data search
        print("1. Testing data search...")
        search_data = {
            "query": "test",
            "offset": 0,
            "count": 10
        }
        response = self._make_request('POST', '/api/dat/search', 
                                    json=search_data)
        if response:
            print("✓ Data search endpoint accessible")
        else:
            print("✗ Data search endpoint failed")
        
        # Test data view (with a test ID)
        print("2. Testing data view...")
        response = self._make_request('GET', '/api/dat/view', 
                                    params={'id': 'test123'})
        if response:
            print("✓ Data view endpoint accessible")
        else:
            print("✗ Data view endpoint failed")
    
    def test_query_endpoints(self):
        """Test query management endpoints"""
        print("\n=== Testing Query Management Endpoints ===")
        
        # Test query list
        print("1. Testing query list...")
        response = self._make_request('GET', '/api/query/list')
        if response:
            print("✓ Query list endpoint accessible")
        else:
            print("✗ Query list endpoint failed")
        
        # Test query creation
        print("2. Testing query creation...")
        query_data = {
            "title": "Test Query",
            "query": {"test": "data"}
        }
        response = self._make_request('POST', '/api/query/create', 
                                    json=query_data)
        if response:
            print("✓ Query creation endpoint accessible")
        else:
            print("✗ Query creation endpoint failed")
    
    def test_all_endpoints(self):
        """Test all available REST endpoints"""
        print("=== Testing DataFed REST API Endpoints ===")
        
        # Basic connectivity
        if not self.test_health_check():
            print("Cannot proceed with API tests - web service not accessible")
            return False
        
        # Test each category of endpoints
        self.test_user_endpoints()
        self.test_project_endpoints()
        self.test_data_endpoints()
        self.test_query_endpoints()
        
        print("\n=== REST API Testing Complete ===")
        return True

class RepoClient:
    """
    Python client that mimics core service communication with repository services.
    Uses the exact same ZMQ setup and message framing as the core service.
    """
    
    def __init__(self, repo_address, repo_pub_key, core_pub_key, core_priv_key, 
                 timeout_ms=20000, log_level=logging.DEBUG):
        """
        Initialize repository client with exact same parameters as core service.
        
        Args:
            repo_address: Repository server address (e.g., "tcp://repo-server:9000")
            repo_pub_key: Repository server's public key (40-char Z85 encoded)
            core_pub_key: Core service's public key (40-char Z85 encoded) 
            core_priv_key: Core service's private key (40-char Z85 encoded)
            timeout_ms: Connection timeout in milliseconds
            log_level: Logging level
        """
        self.repo_address = repo_address
        self.repo_pub_key = repo_pub_key
        self.core_pub_key = core_pub_key
        self.core_priv_key = core_priv_key
        self.timeout_ms = timeout_ms
        
        # Setup logging
        self.logger = logging.getLogger(__name__)
        self.logger.setLevel(log_level)
        
        # Message type mappings (same as core service)
        self._msg_desc_by_type = {}
        self._msg_desc_by_name = {}
        self._msg_type_by_desc = {}
        
        # Initialize ZMQ context and socket
        self._zmq_ctxt = zmq.Context()
        self._socket = None
        
        # Register protobuf protocols (same as core service)
        self._register_protocols()
        
    def _register_protocols(self):
        """Register protobuf protocols exactly like core service"""
        # Register the same protocols as core service
        self._register_protocol_module(auth)
        self._register_protocol_module(sdms)
        self._register_protocol_module(anon)
        
    def _register_protocol_module(self, msg_module):
        """Register a protobuf module (same as Connection.py)"""
        for name, desc in sorted(msg_module.DESCRIPTOR.message_types_by_name.items()):
            if hasattr(msg_module, '_msg_name_to_type'):
                msg_t = msg_module._msg_name_to_type[name]
                self._msg_desc_by_type[msg_t] = desc
                self._msg_desc_by_name[desc.name] = desc
                self._msg_type_by_desc[desc] = msg_t
        
        # Add debug logging to see what message types are registered
        self.logger.debug(f"Registered message types: {list(self._msg_desc_by_type.keys())}")
        for name, msg_type in getattr(msg_module, '_msg_name_to_type', {}).items():
            if 'Repo' in name:
                self.logger.debug(f"Repository message: {name} = {msg_type}")
    
    def connect(self):
        """
        Establish connection to repository service using EXACT same setup as core service.
        This replicates the TaskWorker.cpp connection logic precisely.
        """
        # Parse repository address (same as AddressSplitter in core service)
        if not self.repo_address.startswith("tcp://"):
            raise Exception("Repository address must start with 'tcp://'")
        
        address_parts = self.repo_address[6:].split(":")
        if len(address_parts) != 2:
            raise Exception("Invalid repository address format")
        
        host = address_parts[0]
        port = int(address_parts[1])
        
        # Create ZMQ socket with EXACT same options as core service
        self._socket = self._zmq_ctxt.socket(zmq.DEALER)
        
        # Set socket options (matching core service SocketOptions)
        self._socket.setsockopt(zmq.TCP_KEEPALIVE, 1)
        self._socket.setsockopt(zmq.TCP_KEEPALIVE_CNT, 20)
        self._socket.setsockopt(zmq.TCP_KEEPALIVE_IDLE, 540)
        self._socket.setsockopt(zmq.TCP_KEEPALIVE_INTVL, 5)
        self._socket.setsockopt(zmq.LINGER, 100)
        
        # Set up Curve encryption (EXACT same as core service)
        # Core service uses: cred_options[CredentialType::PUBLIC_KEY] = core_pub_key
        #                   cred_options[CredentialType::PRIVATE_KEY] = core_priv_key  
        #                   cred_options[CredentialType::SERVER_KEY] = repo_pub_key
        
        try:
            self._socket.setsockopt_string(zmq.CURVE_SECRETKEY, self.core_priv_key)
            self._socket.setsockopt_string(zmq.CURVE_PUBLICKEY, self.core_pub_key)
            self._socket.setsockopt_string(zmq.CURVE_SERVERKEY, self.repo_pub_key)
        except Exception as e:
            raise Exception(f"Failed to set Curve encryption keys: {e}")
        
        # Connect to repository server
        self._socket.connect(self.repo_address)
        
        self.logger.info(f"Connected to repository at {self.repo_address}")
        self.logger.debug(f"Core public key: {self.core_pub_key}")
        self.logger.debug(f"Core private key: {self.core_priv_key}")
        self.logger.debug(f"Repo public key: {self.repo_pub_key}")
    
    def send_repo_request(self, request_msg, timeout_ms=None):
        """
        Send request to repository service using EXACT same message framing as core service.
        This replicates the core service's send/recv logic precisely.
        """
        if not self._socket:
            raise Exception("Not connected to repository service")
        
        timeout = timeout_ms if timeout_ms is not None else self.timeout_ms
        
        # Send message using same framing as core service
        self._send_message(request_msg)
        
        # Receive response with timeout
        return self._receive_message(timeout)
    
    def _send_message(self, message):
        """Send message using exact same framing as core service"""
        # Find message type (same as core service)
        if message.DESCRIPTOR not in self._msg_type_by_desc:
            available_types = [desc.name for desc in self._msg_type_by_desc.keys()]
            raise Exception(f"Attempt to send unregistered message type: {message.DESCRIPTOR.name}. Available types: {available_types}")
        
        msg_type = self._msg_type_by_desc[message.DESCRIPTOR]
        self.logger.debug(f"Sending message: {message.DESCRIPTOR.name} (type: {msg_type})")
        
        # Send using exact same framing as Connection.py
        self._socket.send_string("BEGIN_DATAFED", zmq.SNDMORE)
        route_count = 0
        self._socket.send(struct.pack("!i", route_count), zmq.SNDMORE)
        self._socket.send(b"", zmq.SNDMORE)
        
        correlation_id = str(uuid.uuid4())
        self.logger.debug(f"Sending message with correlation id: {correlation_id}")
        self._socket.send_string(correlation_id, zmq.SNDMORE)
        self._socket.send_string(self.core_pub_key, zmq.SNDMORE)  # Use core pub key
        self._socket.send_string("no_user", zmq.SNDMORE)
        
        # Serialize message
        data = message.SerializeToString()
        data_sz = len(data)
        self.logger.debug(f"Serialized message size: {data_sz} bytes")
        
        # Build message frame (same as core service)
        frame = struct.pack(">LBBH", data_sz, msg_type >> 8, msg_type & 0xFF, 0)
        self.logger.debug(f"Message frame: size={data_sz}, type={msg_type} (0x{msg_type:04x}), frame_bytes={frame.hex()}")
        
        if data_sz > 0:
            self._socket.send(frame, zmq.SNDMORE)
            self._socket.send(data, 0)
        else:
            self._socket.send(frame, zmq.SNDMORE)
            self._socket.send(b"", 0)
    
    def _receive_message(self, timeout_ms):
        """Receive message using exact same framing as core service"""
        # Wait for data with timeout
        ready = self._socket.poll(timeout_ms)
        if ready == 0:
            raise Exception(f"Timeout waiting for response from repository")
        
        self.logger.debug("Received data from repository, parsing message...")
        
        # Receive using exact same framing as Connection.py
        self._socket.recv_string(0)  # null frame
        
        header = ""
        while header != "BEGIN_DATAFED":
            header = self._socket.recv_string(0)
        
        msg = self._socket.recv(0)
        route_count = struct.unpack("!i", msg)[0]
        self.logger.debug(f"Route count: {route_count}")
        
        for i in range(0, route_count):
            self._socket.recv(0)  # route
        
        self._socket.recv(0)  # null_packet
        correlation_id = self._socket.recv_string(0)
        self.logger.debug(f"Received message with correlation id: {correlation_id}")
        key = self._socket.recv_string(0)
        client = self._socket.recv_string(0)
        self.logger.debug(f"Received from client: {client}, key: {key[:10]}...")
        
        # Receive custom frame header
        frame_data = self._socket.recv(0)
        frame_values = struct.unpack(">LBBH", frame_data)
        msg_type = (frame_values[1] << 8) | frame_values[2]
        
        self.logger.debug(f"Received message type: {msg_type} (0x{msg_type:04x}), size: {frame_values[0]}")
        self.logger.debug(f"Frame values: {frame_values}")
        
        if msg_type not in self._msg_desc_by_type:
            available_types = list(self._msg_desc_by_type.keys())
            raise Exception(f"Received unregistered message type: {msg_type} (0x{msg_type:04x}). Available types: {available_types}")
        
        desc = self._msg_desc_by_type[msg_type]
        self.logger.debug(f"Message descriptor: {desc.name}")
        
        if frame_values[0] > 0:
            data = self._socket.recv(0)
            self.logger.debug(f"Received payload: {len(data)} bytes")
            reply = desc._concrete_class()
            reply.ParseFromString(data)
        else:
            self.logger.debug("No payload data")
            reply = desc._concrete_class()
        
        return reply, desc.name, frame_values[3]
    
    # Repository-specific methods (matching core service operations)
    
    def get_data_size(self, record_locations):
        """
        Get size of data records from repository.
        Equivalent to RepoDataGetSizeRequest in core service.
        """
        request = auth.RepoDataGetSizeRequest()
        for loc in record_locations:
            location = request.loc.add()
            location.id = loc['id']
            location.path = loc['path']
        
        reply, msg_type, context = self.send_repo_request(request)
        return reply
    
    def delete_data(self, record_locations):
        """
        Delete data records from repository.
        Equivalent to RepoDataDeleteRequest in core service.
        """
        request = auth.RepoDataDeleteRequest()
        for loc in record_locations:
            location = request.loc.add()
            location.id = loc['id']
            location.path = loc['path']
        
        reply, msg_type, context = self.send_repo_request(request)
        return reply
    
    def create_path(self, path):
        """
        Create storage path on repository.
        Equivalent to RepoPathCreateRequest in core service.
        """
        request = auth.RepoPathCreateRequest()
        request.path = path
        
        reply, msg_type, context = self.send_repo_request(request)
        return reply
    
    def delete_path(self, path):
        """
        Delete storage path from repository.
        Equivalent to RepoPathDeleteRequest in core service.
        """
        request = auth.RepoPathDeleteRequest()
        request.path = path
        
        reply, msg_type, context = self.send_repo_request(request)
        return reply
    
    def close(self):
        """Close connection to repository service"""
        if self._socket:
            self._socket.close()
        if self._zmq_ctxt:
            self._zmq_ctxt.destroy()
    
    def __enter__(self):
        self.connect()
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()

# Usage example
def main():
    import sys
    
    # Parse command line arguments
    test_rest = True
    test_zmq = True
    
    if len(sys.argv) > 1:
        if sys.argv[1] == "--rest-only":
            test_zmq = False
        elif sys.argv[1] == "--zmq-only":
            test_rest = False
        elif sys.argv[1] == "--help":
            print("Usage: python test_client.py [--rest-only|--zmq-only|--help]")
            print("  --rest-only: Test only REST API endpoints (web service)")
            print("  --zmq-only:  Test only ZMQ repository service workers")
            print("  --help:      Show this help message")
            print()
            print("Note: REST API is provided by the web service, not the repository service.")
            print("      Repository service only provides ZMQ-based worker functionality.")
            return
    
    print("=== DataFed Service Testing Suite ===")
    print()
    
    rest_success = False
    if test_rest:
        # Test REST API (web service - separate from repository service)
        print("1. Testing REST API endpoints (web service)...")
        print("   Note: This tests the DataFed web service, not the repository service.")
        rest_client = RestApiClient(base_url="http://localhost:8080")
        rest_success = rest_client.test_all_endpoints()
    else:
        print("Skipping REST API tests (--zmq-only mode)")
    
    if test_rest and test_zmq:
        print("\n" + "="*60)
        print()
    
    # Test ZMQ Repository Service Workers
    if test_zmq:
        print("2. Testing ZMQ Repository Service Workers...")
        print("   This tests the repository service's ZMQ-based worker functionality.")
        
        # Load keys from files
        repo_address = "tcp://localhost:10000"  # Repository service runs on port 10000
        repo_pub_key = load_key_file("/mnt/storage/datafed_rs/DataFed/keys/mock-datafed-core-key.pub")
        core_pub_key = load_key_file("/mnt/storage/datafed_rs/DataFed/keys/mock-datafed-core-key.pub")
        core_priv_key = load_key_file("/mnt/storage/datafed_rs/DataFed/keys/mock-datafed-core-key.priv")
        
        if not all([repo_pub_key, core_pub_key, core_priv_key]):
            print("Error: Could not load all required keys for ZMQ testing")
            print("Skipping ZMQ repository service tests...")
        else:
            try:
                with RepoClient(repo_address, repo_pub_key, core_pub_key, core_priv_key) as repo_client:
                    print("=== Testing Repository Service Workers ===")
                    print()
                    
                    # Test 1: Create storage paths
                    print("1. Testing path creation...")
                    test_paths = [
                        "/mnt/storage/datafed_rs/DataFed/data/repo/test_path_1",
                        "/mnt/storage/datafed_rs/DataFed/data/repo/test_path_2", 
                        "/mnt/storage/datafed_rs/DataFed/data/repo/data/12345"
                    ]
                    
                    for path in test_paths:
                        try:
                            path_reply = repo_client.create_path(path)
                            print(f"✓ Created path: {path}")
                        except Exception as e:
                            print(f"✗ Error creating path {path}: {e}")
                    
                    print()
                    
                    # Test 2: Get data sizes (for existing files)
                    print("2. Testing data size requests...")
                    record_locations = [
                        {'id': 'data/12345', 'path': '/mnt/storage/datafed_rs/DataFed/data/repo/data/12345.dat'},
                        {'id': 'data/67890', 'path': '/mnt/storage/datafed_rs/DataFed/data/repo/data/67890.dat'},
                        {'id': 'data/test', 'path': '/mnt/storage/datafed_rs/DataFed/data/repo/test_path_1/test.dat'}
                    ]
                    
                    try:
                        size_reply = repo_client.get_data_size(record_locations)
                        print(f"✓ Data sizes response: {size_reply}")
                    except Exception as e:
                        print(f"✗ Error getting data sizes: {e}")
                    
                    print()
                    
                    # Test 3: Delete data (cleanup)
                    print("3. Testing data deletion...")
                    try:
                        delete_reply = repo_client.delete_data(record_locations)
                        print(f"✓ Data deletion response: {delete_reply}")
                    except Exception as e:
                        print(f"✗ Error deleting data: {e}")
                    
                    print()
                    
                    # Test 4: Delete paths (cleanup)
                    print("4. Testing path deletion...")
                    for path in test_paths:
                        try:
                            path_reply = repo_client.delete_path(path)
                            print(f"✓ Deleted path: {path}")
                        except Exception as e:
                            print(f"✗ Error deleting path {path}: {e}")
                    
                    print()
                    print("=== Repository Service Worker Tests Complete ===")
            except Exception as e:
                print(f"✗ ZMQ Repository Service not accessible: {e}")
                print("Make sure the repository service is running on port 10000")
                print("and your mock core service is running on port 9999")
    else:
        print("Skipping ZMQ Repository Service tests (--rest-only mode)")
    
    print("\n" + "="*60)
    print("=== Testing Summary ===")
    if test_rest:
        if rest_success:
            print("✓ REST API tests completed (web service)")
        else:
            print("✗ REST API tests failed - web service may not be running")
    
    if test_zmq:
        print("✓ ZMQ Repository Service worker tests attempted")
    
    print("\n=== All Tests Complete ===")
    print()
    print("Note: Repository service only provides ZMQ worker functionality.")
    print("      For REST API testing, you need the separate web service running.")

if __name__ == "__main__":
    main()