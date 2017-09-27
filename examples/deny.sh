#!/bin/bash

function send-transaction {
    curl -H "Content-Type: application/json" -X POST -d @$1 http://127.0.0.1:8000/api/services/ac/v1/ac/transaction
}

send-transaction ./json/deny-1.json
send-transaction ./json/deny-2.json