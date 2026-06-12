package com.yunjii.code;

import android.os.Bundle;
import com.getcapacitor.BridgeActivity;

public class MainActivity extends BridgeActivity {
    @Override
    public void onCreate(Bundle savedInstanceState) {
        registerPlugin(YunJiPlugin.class);
        super.onCreate(savedInstanceState);
    }
}
