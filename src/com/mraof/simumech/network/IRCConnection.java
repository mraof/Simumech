package com.mraof.simumech.network;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.PrintWriter;
import java.net.ConnectException;
import java.net.Socket;
import java.net.UnknownHostException;
import java.util.ArrayList;
import java.util.List;

import com.mraof.simumech.MessageParser;


public class IRCConnection implements Runnable
{
	Socket socket;
	public PrintWriter output;
	public BufferedReader in;
	public String hostname;
	public int port;
	public boolean running = false;

	public MessageQueue queue;
	public MessageParser parser;
	public List<String> channels = new ArrayList<String>();

	public String nick = "Burgundy";
	public String prefix = "$b";

	public IRCConnection(String hostname) 
	{
		this(hostname, 6667);
	}

	public IRCConnection(String hostname, int port)
	{
		this.hostname = hostname;
		this.port = port;
		queue = new MessageQueue(this);
		parser = new MessageParser(this);
	}

	@Override
	public void run() 
	{
		in = null;
		try {
			System.out.println("Connecting to " + hostname + " port " + port);
			socket = new Socket(hostname, port);
			output = new PrintWriter(socket.getOutputStream(), true);
			in = new BufferedReader(new InputStreamReader(socket.getInputStream()));
		} catch (UnknownHostException e) {
			e.printStackTrace();
			return;
		} catch (ConnectException e) {
			System.err.println("Failed to connect: " + e.getMessage());
			return;
		}catch (IOException e) {
			e.printStackTrace();
			return;
		}
		running = true;
		(new Thread(queue)).start();
		(new Thread(parser)).start();

		output.println("USER " + nick + " 0 * :MraofBorg");
		output.println("NICK " + nick);
		try {
			while(running)
			{
				String message = in.readLine();
				parser.add(message);
			}
		} catch(IOException e){
			e.printStackTrace();
		}
		
		output.close();
		try {
			in.close();
			socket.close();
		} catch (IOException e) {e.printStackTrace();}
		
		System.out.println("Quit with message Quit message");


	}


}
