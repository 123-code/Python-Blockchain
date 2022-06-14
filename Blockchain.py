import datetime
import hashlib
# to return block, info in json format 
import json
from flask import Flask, jsonify

print("Python Blockchain")

class PythonBlockchain:
    def __init__(self):
        self.chain  = []
        self.createblock(proof=1,prev_hash='0')

    def createblock(self,proof,prev_hash):
        block = {'index':len(self.chain+1),
        'timestamp':str(datetime.datetime.now())
        ,'prev_hash':prev_hash
        ,'proof':proof
        ,'hash':"0x",
        'data':'Data'}
        self.chain.append(block) 
        return block
        
        

