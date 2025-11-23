openssl genrsa -out ca.key 2048

openssl req -x509 -new -nodes \
  -key ca.key \
  -sha256 -days 3650 \
  -out ca.crt \
  -subj "/CN=dev.x CA"

openssl genrsa -out server.key 2048
openssl req -new -key server.key -out server.csr -config cert.conf

openssl x509 -req \
  -in server.csr \
  -CA ca.crt -CAkey ca.key -CAcreateserial \
  -out server.crt \
  -days 365 -sha256 \
  -extfile cert.conf -extensions v3_req

openssl pkcs8 -topk8 -nocrypt -in server.key -out server.pk8
