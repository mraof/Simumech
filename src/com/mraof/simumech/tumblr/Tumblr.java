package com.mraof.simumech.tumblr;

import java.io.File;
import java.io.FileNotFoundException;
import java.io.IOException;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Scanner;

import com.mraof.simumech.IChat;
import com.mraof.simumech.Main;
import com.mraof.simumech.Util;
import com.tumblr.jumblr.JumblrClient;
import com.tumblr.jumblr.types.AnswerPost;
import com.tumblr.jumblr.types.Post;

public class Tumblr implements IChat, Runnable
{
	String blogUrl = "sbnkalny.tumblr.com";
	JumblrClient client;
	Thread tumblrThread;
	String consumerKey = "";
	String consumerSecret = "";
	String oauthToken = "";
	String oauthTokenSecret = "";
	public Tumblr() 
	{
		loadKeys("tumblr.keys");
		client = new JumblrClient(consumerKey, consumerKey);
		client.setToken(oauthToken, oauthTokenSecret);
		tumblrThread = new Thread(this);
		tumblrThread.start();
		
		//User user = client.user();
	}
	public void randomPost()
	{
		Map<String, Object> params = new HashMap<String, Object>();
		params.put("body", Main.markovChain.randomSentence());
		params.put("type", "text");
		params.put("state", "queue");
		try {
			client.postCreate(blogUrl, params);
		} catch (IOException e) {
			e.printStackTrace();
		}
	}
	public void replyToAsk()
	{
		List<Post> posts = client.blogSubmissions(blogUrl);
		for(Post post : posts)
		{
			System.out.println("[Tumblr] " + post.getPostUrl());
			Map<String, Object> params = new HashMap<String, Object>();
			params.put("type", "answer");
			params.put("answer", Main.markovChain.reply(((AnswerPost)post).getQuestion()));
			params.put("tags", Main.markovChain.reply(((AnswerPost)post).getQuestion()));
			//params.put("question", ((AnswerPost)post).getQuestion());
			//params.put("asking_url", ((AnswerPost)post).getAskingUrl());
			//params.put("asking_name", ((AnswerPost)post).getAskingName());
			params.put("state", "queue");

			try {
				client.postEdit(blogUrl, post.getId(), params);
			} catch (IOException e) {
				e.printStackTrace();
			}
		}
	}
	public void setFromString(String string)
	{
		String[] parts = Util.splitFirst(string);
		String varName = parts[0].toUpperCase();
		string = parts[1];
		switch(varName)
		{
		case "BLOG":
			string = string + ".tumblr.com";
		case "URL":
			blogUrl = string;
			break;
		}
	}

	public void message(String message)
	{
	}
	
	public void command(String message)
	{
		String[] parts = Util.splitFirst(message);
		String commandString = parts[0].toUpperCase();
		message = parts[1];
		switch(commandString)
		{
		case "POST":
			randomPost();
			return;
		case "SET":
			setFromString(message);
			return;
		case "ANSWER":
			replyToAsk();
		}
		
	}
	public void run()
	{
		while(true)
		{
			this.replyToAsk();
			try {
				Thread.sleep(900000);
			} catch (InterruptedException e) {
				break;
			}
		}
		System.out.println("[Tumblr] Thread ending");
	}

	public void quit()
	{
		tumblrThread.interrupt();
	}

	public void loadKeys(String filename)
	{
		Scanner scanner;
		try {
			scanner = new Scanner(new File(filename));
		ArrayList<String> keys = new ArrayList<String>();
		while(scanner.hasNext())
			keys.add(scanner.nextLine());
		scanner.close();
		if(keys.size() < 4)
			return;
		consumerKey = keys.get(0);
		consumerSecret = keys.get(1);
		oauthToken = keys.get(2);
		oauthTokenSecret = keys.get(3);

		} catch (FileNotFoundException e) {
			e.printStackTrace();
		}
	}

}
